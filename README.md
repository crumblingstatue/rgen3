# rgen3

rgen3 is a suite of libraries and utilities to manipulate Generation 3 (Gen3) Pokémon games.

Gen3 games include Pokémon FireRed/LeafGreen and Emerald/Ruby/Sapphire.

Below are the currently existing tools.

## rgen3-save

A library to manipulate save files. It also includes an
[example tool](rgen3-save/examples/fill-pc.rs) that fills your PC boxes with random Pokémon!

![](https://hostr.co/file/970/RAqnagQdDUVh/save2.png) ![](https://hostr.co/file/970/saHSJovLiB6J/save3.png)

Pretty cool, right?

## rgen3-string

A library to encode/decode the proprietary string format used for Gen3. 

# Why not use an existing tool?
I couldn't find an already existing save file manipulation tool that works well on Linux.

# Credits
http://bulbapedia.bulbagarden.net/wiki/Save_data_structure_in_Generation_III was almost the
exclusive source of information used to implement rgen3.
