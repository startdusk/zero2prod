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


### 3.安装 sqlx-cli 工具, [教程](https://zhuanlan.zhihu.com/p/377943210)
sqlx 自带了一个命令行工具，方便我们进行常规 SQL 的操作，如添加表格、添加索引、增减表的列等。

利用 cargo 安装 sqlx-cli 这个工具:
```bash
# supports all databases supported by SQLx
cargo install sqlx-cli

# only for postgres
cargo install sqlx-cli --no-default-features --features postgres
```

使用:
1.先声明数据库链接地址
```bash
export DATABASE_URL=postgres://postgres:password@127.0.0.1:5432/newsletter
```
2.创建数据库迁移文件
```bash
sqlx migrate add <迁移文件件名>
```
然后会在项目根目录下生成一个migrations/<时间戳-迁移文件名>
然后，在这个文件里面添加你要执行SQL语句
如:
```sql
CREATE TABLE subscriptions(
	id uuid NOT NULL,
	PRIMARY KEY (id),
	email TEXT NOT NULL UNIQUE,
	name TEXT NOT NULL,
	subscribed_at timestamptz NOT NULL
);
```

3.执行数据库迁移命令
```bash
sqlx migrate run
```

### 4.Docker 打包Rust镜像使用rust:x.xx.x-alpine会更小，但交叉编译成linux 需要使用 `rust-musl-builder`