# radon 
- **r**eally **a**wesome meta package manager f**o**r **n**ix systems (couldnt afford the d)
- formerly wolfram/scheele
- current version 3.0 - [puffy](https://en.wikipedia.org/wiki/OpenBSD#Songs_and_artwork)

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

# Next Point Release Goals

- pkgbuild and ebuild support
- honestly i dont have much ideas
- maybe windows support???

# Installation

- `sudo pacman -S cargo git patch`
- or
- `doas apk add cargo git patch`
- or
- `pkg install cargo git patch`
- and
- `git clone https://github.com/tungstencube-git/radon`
- `cd radon`
- `cargo build --release`
- autotools make meson ninja nimble nim stack cmake optional but you most likely will need them

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
- how does this compare to pkgbuild or ebuild - both pkgbuild and ebuild need their own build file, radon does not

# Misc

- [CONTRIBUTING.md](CONTRIBUTING.md)
-  I am not responsible for bricked devices, dead HDDs, or thermonuclear war.
