use std::io::{self, Write};

use spatio::client::{CliArgs, ClientConnection, OutputFormatter};
use spatio::Result;

fn main() -> Result<()> {
    let args = CliArgs::parse_args();

    // 验证参数
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // 创建连接
    let mut connection = ClientConnection::new(&args.host, args.port);

    if args.should_run_interactive() {
        // 交互模式
        run_interactive_mode(&mut connection, &args.host, args.port)?;
    } else {
        // 直接命令模式
        run_command_mode(&mut connection, &args.command)?;
    }

    Ok(())
}

fn run_command_mode(connection: &mut ClientConnection, command: &[String]) -> Result<()> {
    // 连接到服务器
    connection.connect()?;

    // 执行命令
    let response = connection.send_command(command)?;

    // 格式化并输出结果
    let formatted = OutputFormatter::format_response(&response);
    println!("{}", formatted);

    // 断开连接
    connection.disconnect()?;

    Ok(())
}

fn run_interactive_mode(connection: &mut ClientConnection, host: &str, port: u16) -> Result<()> {
    println!("spatio-cli interactive mode");
    println!("{}", OutputFormatter::format_connecting_message(host, port));

    // 连接到服务器
    match connection.connect() {
        Ok(_) => println!("{}", OutputFormatter::format_connected_message(host, port)),
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return Ok(());
        }
    }

    println!("Type 'HELP' for available commands, 'QUIT' to exit.");
    println!();

    // 创建标准输入读取器
    let stdin = io::stdin();

    loop {
        // 显示提示符
        print!("{}", OutputFormatter::format_prompt(host, port));
        io::stdout().flush()?;

        // 读取用户输入
        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => {
                // EOF (Ctrl+D)
                println!();
                break;
            }
            Ok(_) => {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                // 解析命令 - 特殊处理 SET 命令的 JSON 参数
                let parts = parse_command_line(input);

                if parts.is_empty() {
                    continue;
                }

                // 处理特殊命令
                match parts[0].to_uppercase().as_str() {
                    "QUIT" | "EXIT" => {
                        // 发送 QUIT 命令到服务器
                        if let Ok(response) = connection.send_command(&["QUIT".to_string()]) {
                            println!("{}", OutputFormatter::format_response(&response));
                        }
                        break;
                    }
                    "HELP" => {
                        println!("{}", OutputFormatter::format_help_message());
                        continue;
                    }
                    _ => {
                        // 执行普通命令
                        match connection.send_command(&parts) {
                            Ok(response) => {
                                println!("{}", OutputFormatter::format_response(&response));
                            }
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                // 如果连接断开，尝试重连
                                if !connection.is_connected() {
                                    println!("Connection lost. Attempting to reconnect...");
                                    match connection.connect() {
                                        Ok(_) => println!(
                                            "{}",
                                            OutputFormatter::format_connected_message(host, port)
                                        ),
                                        Err(e) => {
                                            eprintln!("Failed to reconnect: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    println!("{}", OutputFormatter::format_disconnected_message());
    connection.disconnect()?;

    Ok(())
}

fn parse_command_line(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return Vec::new();
    }

    // 过滤控制字符
    let cleaned_input: String = input
        .chars()
        .filter(|&c| c.is_ascii_graphic() || c == ' ' || c == '\t')
        .collect();

    let parts: Vec<&str> = cleaned_input.splitn(3, ' ').collect();
    if parts.is_empty() {
        return Vec::new();
    }

    let command = parts[0].to_uppercase();

    match command.as_str() {
        "SET" => {
            if parts.len() >= 3 {
                // SET collection id geojson
                let mut result = vec![command, parts[1].to_string()];

                // 对于 SET 命令，将第三个参数后的所有内容作为一个整体
                let remaining = parts[2];

                // 进一步分割 id 和 geojson
                if let Some(space_pos) = remaining.find(' ') {
                    let id = &remaining[..space_pos];
                    let geojson = &remaining[space_pos + 1..];

                    // 移除 geojson 外层的引号（如果有）
                    let geojson = remove_outer_quotes(geojson);

                    result.push(id.to_string());
                    result.push(geojson.to_string());
                } else {
                    // 只有 id，没有 geojson
                    result.push(remaining.to_string());
                }

                result
            } else {
                // 参数不够，按正常方式分割
                cleaned_input
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            }
        }
        "INTERSECTS" => {
            if parts.len() >= 3 {
                // INTERSECTS collection geojson [limit]
                let mut result = vec![command, parts[1].to_string()];

                // 对于 INTERSECTS 命令，第二个参数后的所有内容作为一个整体（可能包含 limit）
                let remaining = parts[2];

                // 尝试从末尾提取可能的 limit 参数
                let remaining_parts: Vec<&str> = remaining.rsplitn(2, ' ').collect();

                if remaining_parts.len() == 2 {
                    // 检查最后一个部分是否是数字（limit）
                    let potential_limit = remaining_parts[0];
                    if potential_limit.parse::<usize>().is_ok() {
                        // 最后一个是 limit，前面的是 geojson
                        let geojson = remaining_parts[1];
                        let geojson = remove_outer_quotes(geojson);
                        result.push(geojson.to_string());
                        result.push(potential_limit.to_string());
                    } else {
                        // 整个都是 geojson，没有 limit
                        let geojson = remove_outer_quotes(remaining);
                        result.push(geojson.to_string());
                    }
                } else {
                    // 整个都是 geojson，没有 limit
                    let geojson = remove_outer_quotes(remaining);
                    result.push(geojson.to_string());
                }

                result
            } else {
                // 参数不够，按正常方式分割
                cleaned_input
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            }
        }
        _ => {
            // 其他命令按空格分割
            cleaned_input
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        }
    }
}

fn remove_outer_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        return &s[1..s.len() - 1];
    }
    s
}
