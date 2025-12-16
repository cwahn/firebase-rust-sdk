fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Download and compile Firestore protobuf definitions
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let proto_dir = out_dir.join("proto");
    
    // Create proto directory structure
    std::fs::create_dir_all(proto_dir.join("google/firestore/v1"))?;
    std::fs::create_dir_all(proto_dir.join("google/api"))?;
    std::fs::create_dir_all(proto_dir.join("google/rpc"))?;
    std::fs::create_dir_all(proto_dir.join("google/type"))?;
    
    // Download proto files from googleapis repository
    let proto_files = vec![
        ("google/firestore/v1/firestore.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/firestore.proto"),
        ("google/firestore/v1/common.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/common.proto"),
        ("google/firestore/v1/document.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/document.proto"),
        ("google/firestore/v1/query.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/query.proto"),
        ("google/firestore/v1/write.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/write.proto"),
        ("google/firestore/v1/aggregation_result.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/aggregation_result.proto"),
        ("google/firestore/v1/bloom_filter.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/bloom_filter.proto"),
        ("google/firestore/v1/explain_stats.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/explain_stats.proto"),
        ("google/firestore/v1/pipeline.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/pipeline.proto"),
        ("google/firestore/v1/query_profile.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/firestore/v1/query_profile.proto"),
        ("google/api/annotations.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/api/annotations.proto"),
        ("google/api/client.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/api/client.proto"),
        ("google/api/field_behavior.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/api/field_behavior.proto"),
        ("google/api/http.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/api/http.proto"),
        ("google/api/launch_stage.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/api/launch_stage.proto"),
        ("google/api/routing.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/api/routing.proto"),
        ("google/rpc/status.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/rpc/status.proto"),
        ("google/type/latlng.proto", "https://raw.githubusercontent.com/googleapis/googleapis/master/google/type/latlng.proto"),
    ];
    
    for (path, url) in proto_files {
        let dest = proto_dir.join(path);
        let content = ureq::get(url).call()?.into_string()?;
        std::fs::write(&dest, content)?;
        println!("cargo:rerun-if-changed={}", dest.display());
    }
    
    // Compile protobuf definitions
    // Must compile all proto files in a single call so cross-package references work
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &[
                proto_dir.join("google/firestore/v1/firestore.proto"),
            ],
            &[proto_dir],
        )?;
    
    // Create a module file that properly structures the generated code
    // The generated google.firestore.v1.rs references:
    // - super::super::super::r#type (from google::firestore::v1)  
    // - super::super::rpc (from google::firestore::v1)
    //
    // So we need this structure:
    // root_module (where we include_proto)
    //   ├── google
    //   │   ├── firestore::v1 (from google.firestore.v1.rs)
    //   │   └── rpc (from google.rpc.rs)
    //   └── r#type (from google.r#type.rs)
    
    // Create a module file with proper structure for cross-package references
    // The generated files contain:
    // - google.firestore.v1.rs: Contains types but references super::super::super::r#type and super::super::rpc
    // - google.rpc.rs: Contains rpc types
    // - google.r#type.rs: Contains type definitions
    //
    // We need to create this structure:
    // proto/
    //   ├── r#type.rs (re-exported from google.r#type.rs)  <- for super::super::super::r#type
    //   └── google/
    //       ├── rpc.rs (from google.rpc.rs)                <- for super::super::rpc
    //       └── firestore/
    //           └── v1.rs (from google.firestore.v1.rs)
    
    let out_dir = std::env::var("OUT_DIR")?;
    let proto_rs = std::path::Path::new(&out_dir).join("proto.rs");
    
    std::fs::write(&proto_rs, format!(r#"
// Root module for proto definitions

// Re-export google.r#type at root level so super::super::super::r#type works from firestore
#[path = "{out_dir}/google.r#type.rs"]
pub mod r#type;

// Google package
pub mod google {{
    // Include RPC definitions
    #[path = "{out_dir}/google.rpc.rs"]
    pub mod rpc;
    
    // Firestore package
    pub mod firestore {{
        // Include v1 definitions
        #[path = "{out_dir}/google.firestore.v1.rs"]
        pub mod v1;
    }}
    
    // Also re-export type at google level for direct access
    pub use super::r#type;
}}
"#))?;
    
    Ok(())
}
