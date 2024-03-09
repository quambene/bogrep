use bogrep::{
    errors::BogrepError, html, Action, BookmarkManager, BookmarkService, Cache, CacheMode, Caching,
    Fetch, MockClient, RunMode, ServiceConfig, Settings, TargetBookmark, TargetBookmarkBuilder,
    TargetBookmarks,
};
use chrono::Utc;
use criterion::{criterion_group, criterion_main, Criterion};
use futures::{stream, StreamExt};
use log::{debug, trace, warn};
use std::{collections::HashMap, error::Error, fs, io::Write, sync::Arc};
use tempfile::tempdir;
use tokio::sync::Mutex;
use url::Url;

fn bench_fetch(c: &mut Criterion) {
    c.bench_function("concurrent 100", |b| {
        let runtime = tokio::runtime::Runtime::new().expect("Can't create tokio runtime");
        b.to_async(runtime).iter(|| fetch_concurrently(100));
    });

    c.bench_function("parallel 100", |b| {
        let runtime = tokio::runtime::Runtime::new().expect("Can't create tokio runtime");
        b.to_async(runtime).iter(|| fetch_in_parallel(100));
    });

    c.bench_function("concurrent 500", |b| {
        let runtime = tokio::runtime::Runtime::new().expect("Can't create tokio runtime");
        b.to_async(runtime).iter(|| fetch_concurrently(500));
    });

    c.bench_function("parallel 500", |b| {
        let runtime = tokio::runtime::Runtime::new().expect("Can't create tokio runtime");
        b.to_async(runtime).iter(|| fetch_in_parallel(500));
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_fetch
);
criterion_main!(benches);

async fn fetch_concurrently(max_concurrent_requests: usize) {
    let now = Utc::now();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let cache_path = temp_path.join("cache");
    fs::create_dir(&cache_path).unwrap();
    assert!(cache_path.exists(), "Missing path: {}", temp_path.display());

    let settings = Settings {
        max_concurrent_requests,
        ..Default::default()
    };
    let config = ServiceConfig::new(
        RunMode::FetchAll,
        &settings.ignored_urls,
        settings.max_concurrent_requests,
    )
    .unwrap();
    let cache = Cache::new(&cache_path, CacheMode::Text);
    let client = MockClient::new();
    let mut bookmark_manager = BookmarkManager::new();

    for i in 0..10000 {
        let url = Url::parse(&format!("https://url{i}.com")).unwrap();
        client.add(CONTENT.to_owned(), &url).unwrap();
        bookmark_manager.target_bookmarks_mut().insert(
            TargetBookmarkBuilder::new(url.clone(), now)
                .with_action(Action::FetchAndReplace)
                .build(),
        );
    }

    assert_eq!(bookmark_manager.target_bookmarks().len(), 10000);

    let service = BookmarkService::new(config, client, cache);

    service
        .process(&mut bookmark_manager, &[], now)
        .await
        .unwrap();
}

async fn fetch_in_parallel(max_parallel_requests: usize) {
    let now = Utc::now();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let cache_path = temp_path.join("cache");
    fs::create_dir(&cache_path).unwrap();
    assert!(cache_path.exists(), "Missing path: {}", temp_path.display());
    let cache = Cache::new(&cache_path, CacheMode::Text);
    let client = MockClient::new();

    let mut bookmarks = HashMap::new();

    for i in 0..10000 {
        let url = Url::parse(&format!("https://url{i}.com")).unwrap();
        client.add(CONTENT.to_owned(), &url).unwrap();
        bookmarks.insert(
            url.clone(),
            TargetBookmarkBuilder::new(url.clone(), now)
                .with_action(Action::FetchAndReplace)
                .build(),
        );
    }

    let bookmarks = TargetBookmarks::new(bookmarks)
        .iter()
        .map(|bookmark| Arc::new(Mutex::new(bookmark.1.clone())))
        .collect::<Vec<_>>();
    assert_eq!(bookmarks.len(), 10000);

    let client = Arc::new(client);
    let cache = Arc::new(cache);

    process_bookmarks_in_parallel(client, cache, &bookmarks, max_parallel_requests, true)
        .await
        .unwrap();
}

pub async fn process_bookmarks_in_parallel(
    client: Arc<impl Fetch + Send + Sync + 'static>,
    cache: Arc<impl Caching + Send + Sync + 'static>,
    bookmarks: &[Arc<Mutex<TargetBookmark>>],
    max_parallel_requests: usize,
    fetch_all: bool,
) -> Result<(), BogrepError> {
    let mut processed = 0;
    let mut cached = 0;
    let mut failed_response = 0;
    let mut binary_response = 0;
    let mut empty_response = 0;
    let total = bookmarks.len();

    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| {
            tokio::spawn(fetch_and_cache_bookmark(
                client.clone(),
                cache.clone(),
                bookmark.clone(),
                fetch_all,
            ))
        })
        .buffer_unordered(max_parallel_requests);

    while let Some(item) = stream.next().await {
        processed += 1;

        print!("Processing bookmarks ({processed}/{total})\r");

        if let Ok(Err(err)) = item {
            match err {
                BogrepError::HttpResponse(ref error) => {
                    // Usually, a lot of fetching errors are expected because of
                    // invalid or outdated urls in the bookmarks, so we are
                    // using a warning message only if the issue is on our side.
                    if let Some(error) = error.source() {
                        if error.to_string().contains("Too many open files") {
                            warn!("{err}");
                        } else {
                            debug!("{err} ");
                        }
                    } else {
                        debug!("{err} ");
                    }

                    failed_response += 1;
                }
                BogrepError::HttpStatus { .. } => {
                    debug!("{err}");
                    failed_response += 1;
                }
                BogrepError::ParseHttpResponse(_) => {
                    debug!("{err}");
                    failed_response += 1;
                }
                BogrepError::BinaryResponse(_) => {
                    debug!("{err}");
                    binary_response += 1;
                }
                BogrepError::EmptyResponse(_) => {
                    debug!("{err}");
                    empty_response += 1;
                }
                BogrepError::ConvertHost(_) => {
                    warn!("{err}");
                    failed_response += 1;
                }
                BogrepError::CreateFile { .. } => {
                    // Write errors are expected if there are "Too many open
                    // files", so we are issuing a warning instead of returning
                    // a hard failure.
                    warn!("{err}");
                    failed_response += 1;
                }
                // We are aborting if there is an unexpected error.
                err => {
                    return Err(err);
                }
            }
        } else {
            cached += 1;
        }

        std::io::stdout().flush().map_err(BogrepError::FlushFile)?;
    }

    println!();
    println!(
        "Processed {total} bookmarks, {cached} cached, {} ignored, {failed_response} failed",
        binary_response + empty_response
    );

    Ok(())
}

async fn fetch_and_cache_bookmark(
    client: Arc<impl Fetch>,
    cache: Arc<impl Caching>,
    bookmark: Arc<Mutex<TargetBookmark>>,
    fetch_all: bool,
) -> Result<(), BogrepError> {
    let mut bookmark = bookmark.lock().await;

    if fetch_all {
        let website = client.fetch(&bookmark).await?;
        trace!("Fetched website: {website}");
        let html = html::filter_html(&website)?;
        cache.replace(html, &mut bookmark).await?;
    } else if !cache.exists(&bookmark) {
        let website = client.fetch(&bookmark).await?;
        trace!("Fetched website: {website}");
        let html = html::filter_html(&website)?;
        cache.add(html, &mut bookmark).await?;
    }

    Ok(())
}

const CONTENT: &str = r#"1994 software engineering bookDesign Patterns: Elements of Reusable Object-Oriented Software(1994) is asoftware engineeringbook describingsoftware design patterns. The book was written byErich Gamma,Richard Helm,Ralph Johnson, andJohn Vlissides, with a foreword byGrady Booch.  The book is divided into two parts, with the first two chapters exploring the capabilities and pitfalls of object-oriented programming, and the remaining chapters describing 23 classicsoftware design patterns. The book includes examples inC++andSmalltalk.It has been influential to the field of software engineering and is regarded as an important source for object-oriented design theory and practice. More than 500,000[citation needed]copies have been sold in English and in 13 other languages. The authors are often referred to as theGang of Four(GoF).[1]Development and publication history[edit]The book started at a birds of a feather (BoF) session atOOPSLA'90, "Towards an Architecture Handbook", run by Bruce Anderson, where Erich Gamma and Richard Helm met and discovered their common interest. They were later joined by Ralph Johnson and John Vlissides.[2]The original publication date of the book was October 21, 1994 with a 1995 copyright, hence it is often cited with a 1995-year, despite being published in 1994. The book was first made available to the public at the OOPSLA meeting held in Portland, Oregon, in October 1994. As of March 2012, the book was in its 40th printing.Introduction[edit]Chapter 1 is a discussion ofobject-orienteddesign techniques, based on the authors' experience, which they believe would lead to good object-oriented software design, including:The authors claim the following as advantages ofinterfacesover implementation:clients remain unaware of the specific types of objects they use, as long as the object adheres to the interfaceclients remain unaware of the classes that implement these objects; clients only know about the abstract class(es) defining the interfaceUse of an interface also leads todynamic bindingandpolymorphism, which are central features of object-oriented programming.The authors refer toinheritanceaswhite-boxreuse, with
white-box referring to visibility, because the internals of parent classes are often visible tosubclasses. In contrast, the authors refer toobject composition(in which objects with well-defined interfaces are used dynamically at runtime by objects obtaining references to
other objects) asblack-boxreusebecause no internal details of composed objects need be visible in the code using them.The authors discuss the tension between inheritance and encapsulation at length and state that in their experience, designers overuse inheritance (Gang of Four 1995:20).  The danger is stated as follows:"Because inheritance exposes asubclassto details of its parent's implementation, it's often said that 'inheritance breaks encapsulation'". (Gang of Four 1995:19)They warn that the implementation of a subclass can become so bound up with the implementation of its parent class that any change in the parent's implementation will force the subclass to change. Furthermore, they claim that a way to avoid this is to inherit only from abstract classes—but then, they point out that there is minimal code reuse.Using inheritance is recommended mainly when adding to the functionality of existing components, reusing most of the old code and adding relatively small amounts of new code.To the authors, 'delegation' is an extreme form of object composition that can always be used to replace inheritance.  Delegation involves two objects: a 'sender' passes itself to a 'delegate' to let the delegate refer to the sender.   Thus the link between two parts of a system are established only at runtime, not at compile-time.  TheCallbackarticle has more information about delegation.The authors also discuss so-called parameterized types, which are also known asgenerics(Ada, Eiffel,Java, C#, VB.NET, and Delphi) or templates (C++).  These allow any type to be defined without specifying all the other types it uses—the unspecified types are supplied as 'parameters' at the point of use.The authors admit that delegation and parameterization are very powerful but add a warning:"Dynamic, highly parameterized software is harder to understand and build than more static software." (Gang of Four 1995:21)The authors further distinguish between 'Aggregation', where one object 'has' or 'is part of' another object (implying that an aggregate object and its owner have identical lifetimes) and acquaintance, where one object merely 'knows of' another object. Sometimes acquaintance is called 'association' or the 'using' relationship. Acquaintance objects may request operations of each other, but they are not responsible for each other.  Acquaintance is a weaker relationship than aggregation and suggests muchlooser couplingbetween objects, which can often be desirable for maximum maintainability in designs.The authors employ the term 'toolkit' where others might today use 'class library', as in C# or Java.  In their parlance, toolkits are the object-oriented equivalent of subroutine libraries, whereas a 'framework' is a set of cooperating classes that make up a reusable design for a specific class of software. They state that applications are hard to design, toolkits are harder, and frameworks are the hardest to design.Patterns by type[edit]Creational[edit]Creational patternsare ones that create objects, rather than having to instantiate objects directly. This gives the program more flexibility in deciding which objects need to be created for a given case.Abstract factorygroups object factories that have a common theme.Builderconstructs complex objects by separating construction and representation.Factory methodcreates objects without specifying the exact class to create.Prototypecreates objects by cloning an existing object.Singletonrestricts object creation for a class to only one instance.Structural[edit]Structural patternsconcern class and object composition. They use inheritance to compose interfaces and define ways to compose objects to obtain new functionality.Adapterallows classes with incompatible interfaces to work together by wrapping its own interface around that of an already existing class.Bridgedecouples an abstraction from its implementation so that the two can vary independently.Compositecomposes zero-or-more similar objects so that they can be manipulated as one object.Decoratordynamically adds/overrides behaviour in an existing method of an object.Facadeprovides a simplified interface to a large body of code.Flyweightreduces the cost of creating and manipulating a large number of similar objects.Proxyprovides a placeholder for another object to control access, reduce cost, and reduce complexity.Behavioral[edit]Mostbehavioral design patternsare specifically concerned with communication between objects.Chain of responsibilitydelegates commands to a chain of processing objects.Commandcreates objects that encapsulate actions and parameters.Interpreterimplements a specialized language.Iteratoraccesses the elements of an object sequentially without exposing its underlying representation.Mediatorallowsloose couplingbetween classes by being the only class that has detailed knowledge of their methods.Mementoprovides the ability to restore an object to its previous state (undo).Observeris a publish/subscribe pattern, which allows a number of observer objects to see an event.Stateallows an object to alter its behavior when its internal state changes.Strategyallows one of a family of algorithms to be selected on-the-fly at runtime.Template methoddefines the skeleton of an algorithm as an abstract class, allowing its subclasses to provide concrete behavior.Visitorseparates an algorithm from an object structure by moving the hierarchy of methods into one object.Reception[edit]In 2005 the ACMSIGPLANawarded that year's Programming Languages Achievement Award to the authors, in recognition of the impact of their work "on programming practice and programming language design".[3]Criticism has been directed at the concept ofsoftware design patternsgenerally, and atDesign Patternsspecifically. A primary criticism ofDesign Patternsis that its patterns are simply workarounds for missing features in C++, replacing elegant abstract features with lengthy concrete patterns, essentially becoming a "human compiler".Paul Grahamwrote:[4]When I see patterns in my programs, I consider it a sign of trouble. The shape of a program should reflect only the problem it needs to solve. Any other regularity in the code is a sign, to me at least, that I'm using abstractions that aren't powerful enough-- often that I'm generating by hand the expansions of some macro that I need to write.Peter Norvigdemonstrates that 16 out of the 23 patterns inDesign Patternsare simplified or eliminated by language features inLisporDylan.[5]Related observations were made by Hannemann andKiczaleswho implemented several of the 23 design patterns using anaspect-oriented programming language(AspectJ) and showed that code-level dependencies were removed from the implementations of 17 of the 23 design patterns and that aspect-oriented programming could simplify the implementations of design patterns.[6]There has also been humorous criticism, such as a show trial at OOPSLA '99 on 3 November 1999,[7][8][a]and a parody of the format, byJim Coplien, entitled "Kansas City Air Conditioner".In an interview with InformIT in 2009, Erich Gamma stated that the book authors had a discussion in 2005 on how they would have refactored the book and concluded that they would have recategorized some patterns and added a few additional ones, such as extension object/interface, dependency injection, type object, and null object. Gamma wanted to remove the Singleton pattern, but there was no consensus among the authors to do so.[9]See also[edit]Notes[edit]References[edit]"#;
