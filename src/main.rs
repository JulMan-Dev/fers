use clap::{arg, Arg, Command};
use std::{
    cell::RefCell,
    rc::Rc,
    env::args,
    fs,
    io::{stdin, stdout},
    str::FromStr
};
use tty::{
    ansi::{NoColor, Style},
    simple::Color4,
};
use types::Value;

use crate::{
    error::ErrorKind,
    runtime::State,
    parser::parse,
    token::TokenList,
    tty::ansi::RESET
};

pub mod device;
pub mod error;
pub mod ops;
pub mod parser;
pub mod position;
pub mod runtime;
pub mod token;
pub mod tty;
pub mod types;
pub mod utils;
pub mod vm;
mod file;

#[doc = r#"Use for debugging the interpreter. Print the value to the console and returns it."#]
#[macro_export]
macro_rules! debug {
    ($val:expr $(,)?) => {
        match $val {
            ref tmp => {
                eprintln!(
                    "[debugging at {}:{}:{}] {} = {:?}",
                    file!(),
                    line!(),
                    column!(),
                    stringify!($val),
                    tmp
                );

                tmp
            }
        }
    };
}

pub static ERROR_STYLE: Style<Color4, NoColor> = Style::empty()
    .with_italic()
    .with_bold()
    .with_red_foreground(false);

fn main() {
    let mut stdout = stdout();
    let stdin = stdin();

    const STYLE: Style<Color4, NoColor> = Style::empty().with_red_foreground(false).with_bold();

    // set_hook(Box::new(|infos| {
    //
    //
    //     let error = {
    //         let mut buf = String::new();
    //
    //         buf.push_str(&match infos.payload().downcast_ref::<&str>() {
    //             Some(s) => s,
    //             None => match infos.payload().downcast_ref::<String>() {
    //                 Some(s) => s,
    //                 None => "Cannot display error",
    //             },
    //         });
    //
    //         if let Some(loc) = infos.location() {
    //             buf.push_str(&format!("\n\npanic occured at {loc}"));
    //         }
    //
    //         let mut out = String::new();
    //
    //         let lines: Vec<&str> = buf.lines().collect();
    //
    //         for (index, l) in lines.iter().enumerate() {
    //             if index == 0 && index == lines.len() - 1 {
    //                 out += &(String::from("├─…─┤ ") + l + "\n");
    //             } else if index == 0 {
    //                 out += &(String::from("├─…─┐ ") + l + "\n");
    //             } else if index == lines.len() - 1 {
    //                 out += &(String::from("├─…─┘ ") + l + "\n");
    //             } else {
    //                 out += &(String::from("│   │ ") + l + "\n");
    //             }
    //         }
    //
    //         while out.ends_with('\n') {
    //             out.pop();
    //         }
    //
    //         out
    //     };
    //
    //     println!(
    //         "{}",
    //         STYLE.apply_to(format!(
    //             "┌ Unhandled internal fatal engine error:\n{}",
    //             error
    //         ))
    //     );
    //
    //     println!(
    //         "{}",
    //         STYLE.apply_to(
    //             "│\n│ This error may be caused because you passed string with too huge chars.\n└ Or you find a bug inside of the interpreter or the parser."
    //                 .into()
    //         )
    //     );
    //
    //     println!(
    //         "\n{}",
    //         STYLE.apply_to("The interpreter cannot continue due to this error.".into())
    //     );
    //
    //     exit(1);
    // }));
    //
    // panic!("start failed, consider restarting...");

    let matches = Command::new("fers")
        .about("🏎️ Fers, blazing fast language about RPN.")
        .version("0.1.0")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("Execute a file")
                .arg(Arg::new("filename").required(true).help("Filename to execute"))
                .arg(arg!(-i --interactive "Leave REPL after"))
                .arg(arg!(-o --output <output> "Define output file for \"write\" instruction (default to stdout)"))
                .arg(arg!(-S --stats "Generates performance report")),
        )
        .subcommand(Command::new("repl").about("Open empty REPL"))
        .get_matches();

    match matches.subcommand() {
        Some(("run", submatches)) => {
            let filename: &String = submatches
                .get_one("filename")
                .expect("filename is required");
            
            let stats = submatches.get_flag("stats");
            
            let body = match fs::read_to_string(filename) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!(
                        "{}Cannot open {filename}: {err}{RESET}",
                        STYLE.get_sequence()
                    );
                    return;
                }
            };
            let body = Rc::from(body);

            let lexer = match TokenList::from_str(&body) {
                Ok(tokens) => tokens,
                Err(err) => {
                    eprintln!(
                        "{}Cannot parse {filename}: {err:?}{RESET}",
                        STYLE.get_sequence()
                    );
                    return;
                }
            };
            
            let parser = match parse(&lexer) {
                Ok(tokens) => tokens,
                Err(err) => {
                    eprintln!("{}Cannot parse {filename}:\n", STYLE.get_sequence());
                    eprintln!("{:#?}", err);
                    return;
                }
            };

            let mut state = State {
                source: body,
                chunk: parser,
                line: 0,
                macros: Default::default(),
                variables: Default::default(),
                writer: Rc::new(RefCell::new(stdout)),
            };
             
            let result = loop {
                let result = state.step();
                    
                if let Err(ref stack) = result {
                    // Cannot continue, graceful exit
                    if matches!(stack.kind(), ErrorKind::EndOfFile) {
                        break Ok(());
                    }    
                        
                    break Err(stack.clone()); 
                }
            };

            if let Err(ref stack) = result {
                println!("{stack:#?}");
                // if let Ok(format) = stack.format_chunk() {
                //     println!("{format}");
                // } else {
                //     println!("Failed to generate error printing.");
                // }
            }
            
            return;
        }
        Some(("repl", submatches)) => {}
        _ => unreachable!(),
    }

    let argv: Vec<_> = args().collect();

    let Some(filename) = argv.get(1) else {
        const ERROR: Style<Color4, NoColor> = Style::empty().with_red_foreground(false);

        ERROR.send_to(&mut stdout).unwrap();
        println!("You must pass a file to run.");
        ERROR.send_reset_to(&mut stdout).unwrap();

        return;
    };

    // if fs::read_to_string(argv.get(1))

    /*

    println!(
        "Fast Maths Interpreter (version 0.1) - Running on {} [{}]",
        get_os(),
        if is_debug() { "+debug" } else { "" }
    );
    print!(">>> ");

    match stdout.flush() {
        Ok(_) | Err(_) => (),
    };

    let lines = stdin.lines();

    for line in lines {
        match line {
            Ok(line) => {
                let start = Instant::now();

                let tokens: TokenList = match line.parse() {
                    Ok(tokens) => tokens,
                    Err(err) => {
                        println!("{}", err);
                        print!(">>> ");

                        match stdout.flush() {
                            Ok(_) | Err(_) => (),
                        };

                        continue;
                    }
                };
                let parsing_time = start.elapsed();

                let result = match tokens.run() {
                    Ok(tokens) => tokens,
                    Err(err) => {
                        println!("{}", err);
                        print!(">>> ");

                        match stdout.flush() {
                            Ok(_) | Err(_) => (),
                        };

                        continue;
                    }
                };

                println!("{}", format_result(&result));
                println!(
                    "{}",
                    ITALIC.apply_to(&format!(
                        "Took {} (parsing took {})",
                        start.elapsed().display_minized(),
                        parsing_time.display_minized()
                    ))
                );

                print!(">>> ");

                drop(result);

                match stdout.flush() {
                    Ok(_) | Err(_) => (),
                };
            }
            Err(err) => {
                println!(
                    "Error will parsing line: {}",
                    ERROR_STYLE.apply_to(&err.to_string())
                );
                print!(">>> ");

                match stdout.flush() {
                    Ok(_) | Err(_) => (),
                };
            }
        }
    }

    */

    // while let Some(Ok(line)) = lines.next() {}
}

pub const ITALIC: Style<NoColor, NoColor> = Style::empty().with_italic();

pub fn format_result(result: &Vec<Value>) -> String {
    if result.len() == 0 {
        return ITALIC.apply_to("<nothing>".into());
    }

    let mut str = if result.len() == 1 {
        format!("{}", result[0])
    } else if result.len() > 1 {
        result.iter().map(|id| format!("{}", id) + ", ").collect()
    } else {
        "".into()
    };

    while str.ends_with(", ") {
        str = str[..str.len() - 2].into();
    }

    str
}
