# BoxRec Heuristic

The BoxRec Heuristic Tool is a glorified web scraper that finds upcoming boxing matches on Betfair, then takes statistics of these boxers from BoxRec, and compares Betfair's odds with our own odds calculated using BoxRec's stats.
Currently BoxRec calculations only support taking the score listed for each boxer on the bout page for them, but it is hoped for this to be customisable in the future.

This program is simply meant as a proof of concept and no liability for anyone contributing to this project shall be accepted for misuse of BoxRec, Betfair, or monetary loss.
The developer(s) of this project do not support your gambling habits, and nor does this tool try and defeat any rate-limiting technologies found on the target websites.

This project was commisioned for personal use of a client but it was agreed that the source code could be shared.
This is my first project using Rust, so I'm sorry to any experienced Rustaceans that are offended by my code.
I willingly accept criticism/feedback.

## Usage

As of the current Minimum Viable Product (MVP) release, the program works as a commandline tool.
The executable does not take any arguments.
As an end user you are safe to filter out STDERR if you wish, though obivously you will no longer know if everything is running smoothly.

A configuration file can be supplied in the same directory as the executable.
All of the fields are options and will assume default values if they are not provided.
The file is in the YAML format and the default configuration is below (the order of entries does not matter):

```yaml
username: 
password: 
cache_path: ./.cache
request_timeout: 500
notify_threshold: 15
```

Some notes:

* If you explicity set an empty cache path, to-file caching will not be used, though this isn't recommended as it can mean you have to do **a lot** of reCAPTCHAs

* It is strongly recommended that you **do not** modify any files in the cache directory. You can delete them if you want, but trying to edit things yourself can cause jank I'm sure

* You can't just specify a password, it will be ignored if there is no username option

* Yes, the password is stored plaintext. It is not required, just an option there for the lazy

* `request_timeout` expects a positive integer, and is measured in milliseconds. This is the delay between each web request sent to BoxRec (only one is sent per run to Betfair)

* `notify_threshold` expects a positive number between 0 and 100, as it's a percentage. If your odds of winning are `notify-threshold` larger than Betfair's you're notified
