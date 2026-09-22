use cobra_diagnostics_telemetry::{render_lex_errors, render_parse_errors};
use cobra_lexer_quantum::tokenize;
use cobra_parser_ast::parse;
use cobra_runtime_core::interpret;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());

    match command.as_str() {
        "run" => {
            let Some(path) = args.next() else {
                eprintln!("usage: cobrac run <file.cbr>");
                return ExitCode::from(2);
            };
            match read_program(&path).and_then(|source| compile(&source)) {
                Ok(program) => match interpret(&program) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(error) => {
                        eprintln!("runtime error: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        "check" => {
            let Some(path) = args.next() else {
                eprintln!("usage: cobrac check <file.cbr>");
                return ExitCode::from(2);
            };
            match read_program(&path).and_then(|source| compile(&source)) {
                Ok(_) => {
                    println!("ok: {path}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        "tokens" => {
            let Some(path) = args.next() else {
                eprintln!("usage: cobrac tokens <file.cbr>");
                return ExitCode::from(2);
            };
            match read_program(&path)
                .and_then(|source| tokenize(&source).map_err(|errors| render_lex_errors(&errors)))
            {
                Ok(tokens) => {
                    for token in tokens {
                        println!("{:?}  {}", token.kind, token.span.line);
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        "ast" => {
            let Some(path) = args.next() else {
                eprintln!("usage: cobrac ast <file.cbr>");
                return ExitCode::from(2);
            };
            match read_program(&path).and_then(|source| compile(&source)) {
                Ok(program) => {
                    println!("{program:#?}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        unknown => {
            eprintln!("unknown command `{unknown}`\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn read_program(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("could not read `{path}`: {error}"))
}

fn compile(source: &str) -> Result<cobra_parser_ast::Program, String> {
    let tokens = tokenize(source).map_err(|errors| render_lex_errors(&errors))?;
    parse(tokens).map_err(|errors| render_parse_errors(&errors))
}

fn print_help() {
    println!(
        "Cobra compiler and runtime\n\n\
         usage:\n\
           cobrac run <file.cbr>       execute a Cobra program\n\
           cobrac check <file.cbr>     validate source without running it\n\
           cobrac tokens <file.cbr>    print the lexer token stream\n\
           cobrac ast <file.cbr>       print the parsed abstract syntax tree"
    );
}
