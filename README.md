# scheduler

> Scheduler? I hardly know 'er! - Brennan

Scheduler is the re-implementation of [CourseUp](https://github.com/VikeLabs/courseup),
but in ✨Rust✨ 🦀🦀🦀 which mean's it's blazingly🔥🔥🔥 fast🚀🚀🚀

# Getting Started

1. Either you received dev AWS creds from Brennan or you'll need to run DynamoDB locally:
```
docker run -p 8000:8000 amazon/dynamodb-local
```

then make sure `USE_LOCAL_DYNAMO` is set on your env

2. Install `cargo-watch`:
```
cargo install cargo-watch
```
3. Scrape some section data
```
cargo run --bin scraper -- --oldest 202401
```
(feel free to adjust the oldest section date)

4. Run the scheduler
```
cargo watch -x run
```
with `DISCORD_SECRET` if you want to test out logging in with Discord:
```
DISCORD_SECRET=... cargo watch -x run
```


## References

[New UI mockup (mobile-first)](https://excalidraw.com/#json=Gy2QfYj48tbT_JeTS-Lju,s66OheueOnmxZdSqZ1CQsw)
[Old UI mockup](https://excalidraw.com/#json=2OjX312ssDChqQvrh2j3v,guHSNiaE7K6ZetynsgEKFw)

[Malcolm's Gist](https://gist.github.com/malcolmseyd/ce59c51d376ec3f6f57e5e2f09f59b9a)
[Brennan's Gist](https://gist.github.com/brennanmcmicking/a76b1556a01b655ad5ca8309a9c646c8)
