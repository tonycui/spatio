use std::io::{self, Write};

use geo42::client::{ClientConnection, CliArgs, OutputFormatter};
use geo42::Result;

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
    println!("geo42-cli interactive mode");
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
                
                // 解析命令
                let parts: Vec<String> = input.split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                
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
                                        Ok(_) => println!("{}", OutputFormatter::format_connected_message(host, port)),
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
