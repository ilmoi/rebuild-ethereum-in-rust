# The project
Toy project to rebuild a (significantly simplified) version of Ethereum in
rust.

Part of my on-going effort to teach myself rust lang. 🦀

# The repo
Repo contains both the js and rs code. The two should mirror each other. 

Feel free to fork / use as you please. 

# The code
JS code is not my own. All credit goes to [David Joseph Katz](https://github.com/15Dkatz) and [his
udemy course](https://www.udemy.com/course/build-ethereum-from-scratch/) 
(original [github repo](https://github.com/15Dkatz/build-ethereum-from-scratch)). Kudos
for the awesome course dude.

Rust code is definitely not production-grade - no edge cases, no error
handling, no proper testing, unwraps() all over the place. I basically only wrote the happy path - not to mention some dubious design/architecture choices...

To see rust code in action, follow the instructions in the below file: 
```
rs/play.http
```
It can be opened directly using any of JetBrains' editors, eg the free edition
of PyCharm or CLion or IntelliJ. Otherwise just follow the logic of the doc and
create requests manually eg in Postman.

Happy learning 🚀
