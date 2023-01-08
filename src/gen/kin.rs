pub mod agora {
    pub mod account {
        pub mod v4 {
            tonic::include_proto!("kin.agora.account.v4");
        }
    }
    pub mod airdrop {
        pub mod v4 {
            tonic::include_proto!("kin.agora.airdrop.v4");
        }
    }
    pub mod common {
        pub mod v3 {
            tonic::include_proto!("kin.agora.common.v3");
        }
        pub mod v4 {
            tonic::include_proto!("kin.agora.common.v4");
        }
    }
    pub mod transaction {
        pub mod v4 {
            tonic::include_proto!("kin.agora.transaction.v4");
        }
    }
}
