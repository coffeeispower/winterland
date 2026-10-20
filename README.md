<p align=center>
<img width="300" src="https://raw.githubusercontent.com/coffeeispower/winterland/refs/heads/master/winterland.jpg"/>
</p>
<h1 align=center border=none>Winterland</h1>

<h3 align=center border=none>‼️ WARNING: This project is in an early and unfinished state ‼️</h3>


Winterland is a refreshing new tool for working with your nix flakes built as the next generation of [`nix-output-monitor` (aka nom)](https://github.com/maralorn/nix-output-monitor).

Inspect, visualize, debug, and build flake outputs through an interactive GUI:
- **Browse flake outputs:** Explore your flake's outputs in a tree and select what you want to build with a click instead of typing long attribute paths by hand.
- **Visualize the dependency graph:** See what's already cached, what needs to be built, and uncover unexpected dependencies before they waste your time.
- **Debug your nix code instead of your nix knowledge:** Follow build progress in real time, see which derivations are blocked by others, and inspect logs for individual packages instead of digging through one massive build log.
- **Get straight to the problem:** Inspect derivation details and Nix store paths to investigate failures and understand how your build fits together.

Available for Linux and MacOS.