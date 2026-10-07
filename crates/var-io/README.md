# var-io

This crate implements a small subset of logic similar to Google's Protobuf
spec, so it can be used to interface with Minecraft (Java Edition) clients over
the network.

> [!WARNING]
> This crate is only meant to be used by its parent obServer project.
> Hence, do not assume any sort of compliance to the official Protobuf.

## What's provided

Specifically, this crate adds support for writing `VarInt`s and `VarString`s. These are not real types added by this crate--rather, they are _ways_ to represent integers and strings when serialized to a byte stream.

This is done by defining 2 supertraits: `VarRead<T: Read>` and `VarWrite<T: Write>`. These provide blanket implementations for all types that implement the `Read`/`Write` trait, exposing ready-to-use methods like `read_var_int`, `write_var_int`, `read_var_string`, and `write_var_string`. 

Additional methods are included specifically for transmitting and receiving packets from a byte stream (see the [Minecraft Java edition network packet protocol](https://minecraft.wiki/w/Java_Edition_protocol/Packets)).

It is implemented from scratch and carries zero dependencies (yay).
