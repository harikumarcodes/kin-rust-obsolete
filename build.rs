fn main() -> Result<(), Box<dyn std::error::Error>> {
    if build_protos().is_err() {
        get_protos();
        build_protos()?
    }

    Ok(())
}

fn build_protos() -> std::io::Result<()> {
    tonic_build::configure().build_server(false).compile(
        &[
            "proto/account/v4/account_service.proto",
            "proto/airdrop/v4/airdrop_service.proto",
            "proto/transaction/v4/transaction_service.proto",
            "proto/common/v3/model.proto",
            "proto/common/v4/model.proto",
        ],
        &["proto"],
    )
}

fn get_protos() {
    run(&[
        "mkdir proto",
        "git clone https://github.com/kinecosystem/agora-api --depth 1",
        "git clone https://github.com/envoyproxy/protoc-gen-validate --depth 1",
        "cp -r agora-api/proto/* proto",
        "cp -r protoc-gen-validate/validate proto",
        "rm -rf agora-api protoc-gen-validate",
    ]);
}

fn run(commands: &[&str]) {
    for command in commands {
        let parts: Vec<&str> = command.split(' ').collect();
        let program = parts[0];
        let args = &parts[1..parts.len()];

        std::process::Command::new(program)
            .args(args)
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute {}", command));
    }
}
