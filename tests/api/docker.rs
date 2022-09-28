use std::process::Command;
use std::{thread, time};

use anyhow::Ok;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Container tracks information about the docker container started for tests.
pub struct Container {
    pub id: String,
    pub host: String,
    pub port: u16,
}

/// starts the specified container for running tests.
pub fn start_container(
    image: String,
    port: String,
    args: Vec<String>,
) -> Result<Container, anyhow::Error> {
    let output = Command::new("docker")
        .arg("run")
        .arg("-P") // -P: 将容器指定端口随机映射到宿主机一个端口上
        .arg("-d")
        .args(args)
        .arg(&image)
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }
    let output = String::from_utf8(output.stdout)?;

    let id = output[..12].to_string();
    let ns = extract_ip_and_port(id.clone(), port)?;
    let host = format!("{}:{}", ns.host_ip, ns.host_port);

    for i in 1..=10 {
        let output = Command::new("docker")
            .arg("inspect")
            .arg("-f")
            .arg("{{.State.Status}}")
            .arg(&id)
            .output()?;
        let output = String::from_utf8(output.stdout)?;
        let output = output.trim();
        if output == "running" {
            println!(
                r#"
Docker Started
Image:       {image}
ContainerID: {id}
Host:        {host}
                "#
            );
            break;
        } else {
            if i == 10 {
                return Err(anyhow::anyhow!("cannot start the image[{image}] container"));
            }
            println!("Container[{id}] state {output}, Watting for start");
            let ten_millis = time::Duration::from_secs(i);
            thread::sleep(ten_millis);
        }
    }

    Ok(Container {
        id,
        host: ns.host_ip,
        port: ns.host_port.parse::<u16>().unwrap(),
    })
}

/// stops and removes the specified container.
pub fn stop_container(id: String) -> Result<(), anyhow::Error> {
    let output = Command::new("docker").arg("stop").arg(&id).output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }
    println!(r#"Docker Container Stopped: {id}"#);

    let output = Command::new("docker")
        .arg("rm")
        .arg(&id)
        .arg("-v")
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }
    println!(r#"Docker Container Removed: {id}"#);
    Ok(())
}

fn extract_ip_and_port(id: String, port: String) -> Result<NetworkSettings, anyhow::Error> {
    // 这里{}会当做一个插值表达式，如果需要表示一个{}，需要用{{}}，表示{{}}，则用{{{{}}}}
    let tmpl = format!(
        r#"'[{{{{range $k,$v := (index .NetworkSettings.Ports "{port}/tcp")}}}}{{{{json $v}}}}{{{{end}}}}]'"#
    );
    let output = Command::new("docker")
        .arg("inspect")
        .arg("-f")
        .arg(tmpl)
        .arg(&id)
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }

    let json_string = String::from_utf8(output.stdout)?;
    let datas: Vec<NetworkSettings> = serde_json::from_str(&json_string.trim().trim_matches('\''))?;
    assert!(
        datas.len() >= 1,
        "The container[{id}] cannnot find NetworkSettings.Ports"
    );
    let mut network_settings = NetworkSettings::default();
    if let Some(ns) = datas.first() {
        network_settings.host_ip = ns.host_ip.clone();
        network_settings.host_port = ns.host_port.clone();
    }
    Ok(network_settings)
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct NetworkSettings {
    // alias 反序列化时使用
    // rename 序列化成指定名字
    #[serde(alias = "HostIp")]
    host_ip: String,

    #[serde(alias = "HostPort")]
    host_port: String,
}

#[ignore = "for local test"]
#[test]
fn start_and_stop_container() {
    let image = "docker/getting-started".to_string();
    let port = "80".to_string();
    let args: Vec<String> = vec![];
    let container = start_container(image, port, args).unwrap();
    dbg!(&container.id);
    stop_container(container.id).unwrap();
}

#[ignore = "for local test"]
#[test]
fn test_extract_ip_and_port() {
    let id = "dfd60e4ef0c0".to_string();
    let port = "5432".to_string();
    let settings = extract_ip_and_port(id, port).unwrap();

    assert_eq!("0.0.0.0", settings.host_ip);
    assert_eq!("5432", settings.host_port);
}

#[ignore = "for local test"]
#[test]
fn parse_json_string() {
    let json_string = r#"[{"HostIp":"0.0.0.0","HostPort":"5432"}]"#;

    let v: Value = serde_json::from_str(json_string).unwrap();
    assert_eq!("0.0.0.0", v[0]["HostIp"]);
    assert_eq!("5432", v[0]["HostPort"]);
}
