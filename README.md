# radon 
- **r**eally **a**wesome meta package manager f**o**r **n**ix systems (couldnt afford the d)
- formerly wolfram/scheele
- current version 2.0 - [hexley](http://www.hexley.com/)

# Features

- gitlab github and codeberg support
- fast awesome building for cargo make and cmake
- written in rust
- search option wow!!!
- better than old fart [gpm](https://github.com/aerys/gpm) (booo!)
- really cool in general
- did i menntion that its fully written in rust?

# Overall Project Goal(s)

- getting in offical repos
- allat for now

# 3.0 "puffy" goals

- autotools support
- binary tracking with hash for remove
- radon.json dependency file
- remove function overhaul
- general bug fixes and patches

# 4.0 "larry" goals

- nimble and stack support
- improved dependency handling
- bulk build and install
- improved appearence for things like search
- cargo.toml to radon.json utility

# 5.0 "glenda" goals

- cmake/makefile into radon.json utility
- rpmfusion xbps-src and aur packages
- customizable build (wlsroots without xwayland)

# Installation

- `git clone https://github.com/tungstencube-git/radon`
- `cd radon`
- `cargo build --release`
- Dependencies - Rust, Cargo, Make (Optional), CMake (Optional), Ninja (Optional), Meson (Optional)


# Commands

| Command                           | Description                                                                                                |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `radon`                           | alias to `radon help`.                                                                                       |
| `radon install <flags> <package>` | self explanatory.                                                               |
| `radon remove`                    | abolishes package from /usr/local/bin or ~/.local/bin.                                                           |
| `radon search`                    | searches for packages (only github)                                                            |
| `radon help <command>`   | help.                                                                  |
| `radon list`             | lists installed packages
| `radon upgrade`          | upgrades installed packages

# FAQ 

- why would i use this over regular building from source - makes the building process easier and uses less bandwidth
- why is the install function not split into multiple files - lazy
- how does it function - clones repository checks for build system builds clones to /usr/local/bin or ~/.local/bin
- what wm are you using (yes ik my rice is very cool) - i3
- how does this compare to ubi - ubi installs binaries (like choccy) this builds from source

# Misc

- [CONTRIBUTING.md](CONTRIBUTING.md)
# Naming Scheme

- 1.0 - [tuz](https://en.wikipedia.org/wiki/Tux_(mascot)#Tuz_2009)
- 2.0 - [hexley](http://hexley.com)
- 3.0 [puffy](https://en.wikipedia.org/wiki/OpenBSD#Songs_and_artwork) (25-06-10 delayed due to my pc not wanting to POST)
- 4.0 - [larry](https://wiki.gentoo.org/wiki/Larry_the_cow) 
- 5.0 - [glenda](https://en.wikipedia.org/wiki/Plan_9_from_Bell_Labs) 
- 6.0 [clarus](https://en.wikipedia.org/wiki/Dogcow) 
- 7.0 [freedo](https://en.wikipedia.org/wiki/Linux-libre) 

# Related/Similar Projects

- [chocolatey](https://github.com/chocolatey/choco)
- [ubi](https://github.com/houseabsolute/ubi)
- [gpm](https://github.com/aerys/gpm)
- [spine](https://github.com/plyght/spine)
