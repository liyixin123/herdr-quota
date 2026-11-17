# 许可证与第三方声明

## 项目许可证

herdr-quota（含源码仓库全部内容）以 MIT 许可证发布，全文见仓库根目录的 [LICENSE](../LICENSE)。

许可证选择记录：维护者于 2026-11-17 在项目会话中确认采用 MIT。`Cargo.toml` 的
`license = "MIT"`、根目录 LICENSE 与本文件保持一致。

## 源码分发

从本仓库获取的源码（`src/`、`tests/`、`scripts/`、`docs/`、`examples/`、构建文件及
本文件）整体按 MIT 提供；下游复制、修改和再分发时随附 LICENSE 中的版权与许可声明
即可。

## 预编译发行包分发

GitHub Release 提供的 `herdr-quota` 预编译二进制（见[预编译发行包](releases.md)）
同样按 MIT 分发。二进制在构建时静态嵌入了下表列出的第三方 Rust crate；本项目不以
源码形式修改这些组件，它们按各自上游公布的许可条款提供，各版本的完整许可证文本与
版权声明见其上游仓库及 crates.io 对应版本。macOS 构建还静态链接 Rust 标准库及其
随附库（MIT OR Apache-2.0，见 [rust-lang/rust](https://github.com/rust-lang/rust)）。
发行包与源码同在 `liyixin123/herdr-quota` 发布，第三方条款的获取途径对任何拿到
发行包的用户同样可用。

## 第三方 Rust 依赖

清单按 `Cargo.lock`（版本 0.1.1 锁定集）生成；许可证表达式来自 crates.io 发布的
crate 元数据，验证命令与日期见文末。

### 链接进二进制的依赖

直接依赖为 serde、serde_json、sha2、toml_edit，其余为传递依赖。

| crate | 版本 | 许可证 | 上游仓库 |
| --- | --- | --- | --- |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| toml_edit | 0.23.10+spec-1.0.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_datetime | 0.7.5+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_writer | 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| winnow | 1.0.5 | MIT | https://github.com/winnow-rs/winnow |
| winnow | 0.7.15 | MIT | https://github.com/winnow-rs/winnow |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| itoa | 1.0.18 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| zmij | 1.0.23 | MIT | https://github.com/dtolnay/zmij |
| digest | 0.10.7 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits |
| generic-array | 0.14.7 | MIT | https://github.com/fizyk20/generic-array |
| typenum | 1.20.1 | MIT OR Apache-2.0 | https://github.com/paholg/typenum |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils |
| libc | 0.2.191 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |

`libc` 仅经 cpufeatures 进入 `aarch64-apple-darwin`（macOS）构建；Linux musl 构建不
链接它。其余 crate 在两个发行目标中均被链接。

### 仅编译期使用的依赖

以下 crate 只在构建（过程宏与构建期检测）时执行，不出现在发行二进制中；为完整起见
按 `Cargo.lock` 一并列出。

| crate | 版本 | 许可证 | 上游仓库 |
| --- | --- | --- | --- |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| quote | 1.0.47 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| syn | 3.0.9 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| version_check | 0.9.5 | MIT/Apache-2.0 | https://github.com/SergioBenitez/version_check |

## 清单维护

依赖或版本变化时重新核对并更新本文件：

```sh
cargo metadata --locked --format-version 1 \
  | jq -r '.packages[] | [.name, .version, .license, (.repository // "-")] | @tsv'
```

本清单于 2026-11-17 按上述命令与 `Cargo.lock` 核对生成。
