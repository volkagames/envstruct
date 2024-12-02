#![allow(dead_code)]
use envstruct::prelude::*;
use serial_test::*;
use std::{env, num::NonZeroU64};

#[test]
#[serial]
fn test_non_zero() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub non_zero_number: NonZeroU64,
    }

    // expect ok
    {
        env::set_var("TEST_NON_ZERO_NUMBER", "42");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.non_zero_number, NonZeroU64::new(42).unwrap());
    }

    // expect error
    {
        env::set_var("TEST_NON_ZERO_NUMBER", "0");

        let parse_result = Config::with_prefix("TEST");
        assert!(parse_result.is_err());
        // source = InvalidVarFormat
        assert!(matches!(
            parse_result.err().unwrap(),
            envstruct::EnvStructError::ParseEnvError { .. }
        ));
    }

    // expect error
    {
        env::set_var("TEST_NON_ZERO_NUMBER", "-1");

        let parse_result = Config::with_prefix("TEST");
        assert!(parse_result.is_err());
        // source = InvalidDigit
        assert!(matches!(
            parse_result.err().unwrap(),
            envstruct::EnvStructError::ParseEnvError { .. }
        ));
    }
}
