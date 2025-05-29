# radon 
- **r**eally **a**wesome meta package manager f**o**r **n**ix systems (couldnt afford the d)
- formerly wolfram/scheele
- current version 1.0 - tuz (not tux!!!) 

# Features

- gitlab github and codeberg support
- fast awesome building for cargo make and cmake
- written in rust
- search option wow!!!
- better than old fart [gpm](https://github.com/aerys/gpm) (booo!)
- really cool in general
- did i menntion that its fully written in rust?

# Overall Project/Next Few Point Release Goals

- getting in offical repos
- binary tracking
- radon specific dependency file
- maybe a naming system like debian
- honestly thats all for now lol

# Next Point Release Goals

- meson ninja autotools support
- search function overhaul
- upgrade command

# Installation

- `git clone https://github.com/tungstencube-git/radon`
- `cd radon`
- `cargo build --release`

# Commands

| Command                           | Description                                                                                                |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `radon`                           | alias to `radon help`.                                                                                       |
| `radon install <flags> <package>` | self explanatory.                                                               |
| `radon remove`                    | abolishes package from /usr/local/bin or ~/.local/bin.                                                           |
| `radon search`                    | searches for packages (only github)                                                            |
| `radon help <command>`            | help.                                                                  |

# FAQ 

- why would i use this over regular building from source - makes the building process easier and uses less bandwidth
- why is the install function not split into multiple files - lazy
- how does it function - clones repository checks for build system builds clones to /usr/local/bin or ~/.local/bin
- what wm are you using (yes ik my rice is very cool) - i3
- how does this compare to ubi - ubi installs binaries (like choccy) this builds from source
