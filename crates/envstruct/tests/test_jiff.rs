#![cfg(feature = "jiff")]

use envstruct::{jiff, prelude::*};
use serial_test::*;
use std::env;

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

#[test]
#[serial]
fn test_jiff_date_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub timestamp: jiff::Timestamp,
        pub zoned: jiff::Zoned,
        pub datetime: jiff::civil::DateTime,
        pub date: jiff::civil::Date,
        pub time: jiff::civil::Time,
    }

    // valid value
    {
        clean_env();
        env::set_var("TEST_TIMESTAMP", "2024-08-19T12:34:56+03:00");
        env::set_var("TEST_ZONED", "2024-08-19T12:34:56+03:00[Europe/Moscow]");
        env::set_var("TEST_DATETIME", "2024-08-19 12:34:56");
        env::set_var("TEST_DATE", "2024-08-19");
        env::set_var("TEST_TIME", "12:34:56");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.timestamp.as_second(), 1724060096);
        assert_eq!(config.zoned.timestamp().as_second(), 1724060096);
        assert_eq!(config.datetime.to_string(), "2024-08-19T12:34:56");
        assert_eq!(config.date.to_string(), "2024-08-19");
        assert_eq!(config.time.to_string(), "12:34:56");
    }

    // values are trimmed before parsing
    {
        clean_env();
        env::set_var("TEST_TIMESTAMP", "  2024-08-19T12:34:56Z  ");
        env::set_var("TEST_ZONED", "2024-08-19T12:34:56+03:00[Europe/Moscow]");
        env::set_var("TEST_DATETIME", "2024-08-19 12:34:56");
        env::set_var("TEST_DATE", "2024-08-19");
        env::set_var("TEST_TIME", "12:34:56");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.timestamp.as_second(), 1724070896);
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_TIMESTAMP", "42");
        env::set_var("TEST_ZONED", "42");
        env::set_var("TEST_DATETIME", "42");
        env::set_var("TEST_DATE", "42");
        env::set_var("TEST_TIME", "42");
        assert!(Config::with_prefix("TEST").is_err());
    }

    // a bare offset resolves to a fixed zone, keeping the offset intact
    {
        clean_env();
        env::set_var("TEST_TIMESTAMP", "2024-08-19T12:34:56Z");
        env::set_var("TEST_ZONED", "2024-08-19T12:34:56+03:00");
        env::set_var("TEST_DATETIME", "2024-08-19 12:34:56");
        env::set_var("TEST_DATE", "2024-08-19");
        env::set_var("TEST_TIME", "12:34:56");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.zoned.timestamp().as_second(), 1724060096);
        assert_eq!(
            config.zoned.to_string(),
            "2024-08-19T12:34:56+03:00[+03:00]"
        );
    }

    // negative offsets and `Z` go down the same path
    {
        clean_env();
        env::set_var("TEST_TIMESTAMP", "2024-08-19T12:34:56Z");
        env::set_var("TEST_DATETIME", "2024-08-19 12:34:56");
        env::set_var("TEST_DATE", "2024-08-19");
        env::set_var("TEST_TIME", "12:34:56");

        env::set_var("TEST_ZONED", "2024-08-19T12:34:56-05:30");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.zoned.timestamp().as_second(), 1724090696);

        env::set_var("TEST_ZONED", "2024-08-19T12:34:56Z");
        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.zoned.to_string(), "2024-08-19T12:34:56+00:00[UTC]");
    }

    // a datetime with neither a zone nor an offset is not a zoned datetime
    {
        clean_env();
        env::set_var("TEST_TIMESTAMP", "2024-08-19T12:34:56Z");
        env::set_var("TEST_ZONED", "2024-08-19T12:34:56");
        env::set_var("TEST_DATETIME", "2024-08-19 12:34:56");
        env::set_var("TEST_DATE", "2024-08-19");
        env::set_var("TEST_TIME", "12:34:56");
        assert!(Config::with_prefix("TEST").is_err());
    }
}

#[test]
#[serial]
fn test_jiff_duration_values() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub span: jiff::Span,
        pub duration: jiff::SignedDuration,
    }

    // the friendly syntax, as humantime spells it
    {
        clean_env();
        env::set_var("TEST_SPAN", "1h 30m");
        env::set_var("TEST_DURATION", "1h 30m");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.span.to_string(), "PT1H30M");
        assert_eq!(config.duration.as_secs(), 5400);
    }

    // ISO-8601 is accepted by the same parser
    {
        clean_env();
        env::set_var("TEST_SPAN", "PT1H30M");
        env::set_var("TEST_DURATION", "PT1H30M");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.span.to_string(), "PT1H30M");
        assert_eq!(config.duration.as_secs(), 5400);
    }

    // days are exactly 24h: a config value has no moment to be relative to,
    // so the DST-dependent meaning of a calendar day never applies here
    {
        clean_env();
        env::set_var("TEST_SPAN", "1d");
        env::set_var("TEST_DURATION", "1d");

        let config = Config::with_prefix("TEST").unwrap();
        assert_eq!(config.span.to_string(), "P1D");
        assert_eq!(config.duration.as_secs(), 86400);

        env::set_var("TEST_DURATION", "7d");
        assert_eq!(
            Config::with_prefix("TEST").unwrap().duration.as_secs(),
            604800
        );

        env::set_var("TEST_DURATION", "1d 12h");
        assert_eq!(
            Config::with_prefix("TEST").unwrap().duration.as_secs(),
            129600
        );
    }

    // months and years are plain duration units, expanded to the same
    // gregorian averages humantime uses: 1mo is 30.44d, 1y is 365.25d
    {
        clean_env();
        env::set_var("TEST_SPAN", "1h");

        for (value, expected) in [
            ("1mo", 2630016),
            ("1y", 31557600),
            ("12mo", 31560192),
            ("1w", 604800),
            ("1y 6mo", 47337696),
            ("1mo 15d", 3926016),
            ("-1mo", -2630016),
        ] {
            env::set_var("TEST_DURATION", value);
            assert_eq!(
                Config::with_prefix("TEST").unwrap().duration.as_secs(),
                expected,
                "{value}"
            );
        }
    }

    // the same string must not mean two different things across the crate
    #[cfg(feature = "humantime")]
    {
        use std::str::FromStr;

        clean_env();
        env::set_var("TEST_SPAN", "1h");

        for value in ["1d", "7d", "1month", "1year", "1h 30m"] {
            env::set_var("TEST_DURATION", value);
            let jiff = Config::with_prefix("TEST").unwrap().duration;
            let humantime = humantime::Duration::from_str(value).unwrap();
            assert_eq!(
                jiff.as_secs() as u64,
                humantime.as_secs(),
                "{value} differs from humantime"
            );
        }
    }

    // invalid value
    {
        clean_env();
        env::set_var("TEST_SPAN", "not a span");
        env::set_var("TEST_DURATION", "not a duration");
        assert!(Config::with_prefix("TEST").is_err());
    }
}

#[test]
#[serial]
fn test_jiff_defaults_and_usage() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "2024-08-19T12:34:56Z")]
        pub timestamp: jiff::Timestamp,

        #[env(default = "1h 30m")]
        pub span: jiff::Span,
    }

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.timestamp.as_second(), 1724070896);
    assert_eq!(config.span.to_string(), "PT1H30M");

    let usage = Config::usage_with_prefix("TEST").unwrap();
    println!("usage: \n{usage}");
    assert_eq!(
        usage,
        concat!(
            " NAME           | TYPE      | DEFAULT\n",
            "----------------+-----------+------------------------\n",
            " TEST_TIMESTAMP | Timestamp | \"2024-08-19T12:34:56Z\"\n",
            " TEST_SPAN      | Span      | \"1h 30m\"\n",
        )
    );
}

#[test]
#[serial]
fn test_jiff_collections() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub timestamps: Vec<jiff::Timestamp>,
        pub spans: Vec<jiff::Span>,
    }

    clean_env();
    env::set_var(
        "TEST_TIMESTAMPS",
        "2024-08-19T12:34:56Z,2024-08-20T12:34:56Z",
    );
    env::set_var("TEST_SPANS", "1h,30m");

    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.timestamps.len(), 2);
    assert_eq!(config.timestamps[0].as_second(), 1724070896);
    assert_eq!(config.timestamps[1].as_second(), 1724157296);
    assert_eq!(config.spans[0].to_string(), "PT1H");
    assert_eq!(config.spans[1].to_string(), "PT30M");
}
