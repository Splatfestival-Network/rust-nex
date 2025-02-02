
fn main(){
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &["grpc-protobufs/account/account_service.proto"],
            &["grpc-protobufs/account"]
        )
        .unwrap();
}