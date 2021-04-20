# Secret Contract Development Toolkit

This repository is a collection of Rust packages that contain common tools used in development of
[Secret Contracts](https://build.scrt.network/dev/secret-contracts.html) running on the
[Secret Network](https://scrt.network/).

While the packages in this repository are designed with Secret Network's runtime in mind, some
of them may work well with the vanilla [CosmWasm](https://cosmwasm.com/) libraries and runtimes
as well, or only require minimal modifications to be compatible with them.

The main package in this repository is `secret-toolkit` under `packages/toolkit`, which is
a wrapper around the other packages. For example `secret-toolkit-storage` is exported under
`secret_toolkit::storage`. If you only need some of the tools from the toolkit, you may get
better compile times by depending on the different components directly.

## License

The license file in the top directory of this repository applies to all packages it contains.
