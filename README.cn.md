### 1.rust编译默认的编译器编译很慢是慢在链接期，使用其他链接器提升编译速度(看情况使用，如果本来编译就不慢的话就不需要使用了)
```bash
# 在项目根目录下 新建 .cargo/config.toml

# On Windows
# ```
# cargo install -f cargo-binutils
# rustup component add llvm-tools-preview
# ```
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# On Linux:
# - Ubuntu, `sudo apt-get install lld clang`
# - Arch, `sudo pacman -S lld clang`
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "linker=clang", "-C", "link-arg=-fuse-ld=lld"]

# On MacOS, `brew install michaeleisel/zld/zld`
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]

```

### 2.使用编译热更新工具 [cargo-watch](https://crates.io/crates/cargo-watch)
```bash
# 安装
cargo install cargo-watch

# 执行命令 重新云心
cargo watch -x check

# 或 热更新 先测试 在运行
cargo watch -x check -x test -x run
```

