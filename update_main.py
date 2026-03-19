with open("src/main.rs", "r") as f:
    code = f.read()

# add debug flag
code = code.replace("pub day_first: bool,", "pub day_first: bool,\n\n    #[arg(short, long)]\n    pub debug: bool,")
code = code.replace("mock_now: None }", "mock_now: None, debug: args.debug }")
code = code.replace("mock_now: Some(mock_date),", "mock_now: Some(mock_date),\n            debug: false,")

with open("src/main.rs", "w") as f:
    f.write(code)
