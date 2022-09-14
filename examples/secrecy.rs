use secrecy::{Secret, ExposeSecret};

// secrecy 这个库非常简单，简单说，就是一个 wrapper
// 这个类型默认故意不实现 std::fmt::Display。所以，如果你要用一个属性，需要显示声明，例如如果需要使用这个 setting 的 password
// secrecy 提供的好处（如果你认为这是好处），即把更多的业务的信息和意图写在代码里。
// 密码和用户名都是字符串，编程语言并不知道密码和用户名有啥不一样，都是字符串。
// 但是类型编程就不一样，类型编程在定义类型时，本质是在造 dsl，或者说在用语言构建一系列有业务含义的代码。
// 并且 rust 本身还能给这些类型提供技术上的支持，并且是非常细粒度的支持，你定义了一个字符串，你大概就知道这个字符串在程序里整个生命周期的情况，你每一个点都想的很清楚，最后 compose 出一个超级复杂且超级健壮的程序。
#[derive(serde::Deserialize)]
pub struct Settings {
	pub password: Secret<String>,
}

impl Settings {
	pub fn connection_string(&self) -> Secret<String> {
		Secret::new(format!(
			"{}",
			self.password.expose_secret(), // 需要显示声明，才能获取password的值
		))
	}
}


fn main() {
	let	password = Secret::new("123456".to_string());
	// println!("{}", password); // Secret 没有实现 fmt::Display, 所以不能打印出来
	println!("{}", password.expose_secret());
}


