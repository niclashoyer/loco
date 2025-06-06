# rust loco

Crates and components for model railroad control. This project started as a rust
learning excercise around christmas to learn embedded rust while the model train ran
around the christmas tree.

It was originally based on `embedded-hal` 0.2.7. Unfortunately a lot of traits (especially timers) got removed for the final 1.0.0 release. A partial port to async rust (with embassy) is in the `async` branch.

Tests where added on the go. Take this project as it is: a learning excercise and a playground for model railroads ☺️

| crate         | description |
| ------------- | ------------- |
| [core](./core) | core traits and types used by all other crates (e.g. addresse, functions, macros)  |
| [dcc](./dcc) | [Digital Command Control](https://en.wikipedia.org/wiki/Digital_Command_Control) (DCC) driver implementation |
| [susi](./susi) | [Serial User Standard Interface](https://dccwiki.com/SUSI) (SUSI) driver implementation |
| [xpressnet](./xpressnet) | [XpressNet](https://dccwiki.com/XpressNet) driver implementation |
| [z21](./z21) | partial [Z21 LAN Protocol](https://www.z21.eu/media/Kwc_Basic_DownloadTag_Component/root-en-main_47-1652-959-downloadTag-download/default/d559b9cf/1628743384/z21-lan-protokoll-en.pdf) implementation based en `embedded-nal` |
| [command-station](./command-station) | basic command station implementation with a [Raspberry Pi example](./command-station/examples/linux-dcc/) |
