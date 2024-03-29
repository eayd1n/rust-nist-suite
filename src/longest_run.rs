//! This module performs the Longest Run of Ones in a Block test. For this test, it is crucial to pass
//! least 128 bit!
//!
//! Description of test from NIST SP 800-22:
//!
//! "The focus of the test is the longest run of ones within M-bit blocks. The purpose of this test is to
//! determine whether the length of the longest run of ones within the tested sequence is consistent with the
//! length of the longest run of ones that would be expected in a random sequence. Note that an irregularity in
//! the expected length of the longest run of ones implies that there is also an irregularity in the expected
//! length of the longest run of zeroes. Therefore, only a test for ones is necessary."

use crate::constants;
use crate::customtypes;
use crate::utils;
use anyhow::{Context, Result};
use std::collections::BTreeMap;

const TEST_NAME: customtypes::Test = customtypes::Test::LongestRun;

/// Perform the Longest Run of Ones in a Block test.
///
/// # Arguments
///
/// bit_string -  The bit string to be tested for randomness
///
/// # Return
///
/// Ok(p-value) - The p-value which indicates whether randomness is given or not
/// Err(err) - Some error occured
pub fn perform_test(bit_string: &str) -> Result<f64> {
    log::trace!("longest_run::perform_test()");

    // capture the current time before executing the actual test
    let start_time = std::time::Instant::now();

    // check if bit string contains invalid characters
    let length = utils::evaluate_bit_string(TEST_NAME, bit_string, constants::MIN_LENGTH)
        .with_context(|| "Invalid character(s) in passed bit string detected")?;

    // evaluate bit string length and determine longest run configuration
    let config = get_longest_run_config(length).with_context(|| {
        format!(
            "{}: Failed to retrieve longest run configuration",
            TEST_NAME
        )
    })?;

    // determine the number of runs per block and calculate v_i. A "longest" run is defined as the
    // maximum number of consecutive ones in a block, e.g., "110010111" has the longest run as of 3
    let mut counts: BTreeMap<i32, i32> = BTreeMap::new();

    for block_num in 0..config.number_of_blocks {
        let start_index = block_num * config.block_size;
        let end_index = (block_num + 1) * config.block_size;
        let block = &bit_string[start_index..end_index];
        let max_consecutive_ones = count_max_consecutive_ones(block);

        *counts.entry(max_consecutive_ones).or_insert(0) += 1;
    }

    log::debug!("{}: Number of runs before merge: {:?}", TEST_NAME, counts);
    let vi_counts = calculate_vi_values(counts, config.thresholds);
    log::debug!("{}: Number of runs after merge: {:?}", TEST_NAME, vi_counts);

    // Now we need to compute chi_square value
    let mut chi_square = 0.0;

    // iterate over vi_values and pi_values at the same time because both have same size
    for ((_, vi_value), &pi_value) in vi_counts.iter().zip(config.pi_values.iter()) {
        log::trace!(
            "{}: Current vi_value: {}, current pi_value: {}",
            TEST_NAME,
            *vi_value,
            pi_value
        );

        let constant = (config.number_of_blocks as f64) * pi_value;
        chi_square += ((*vi_value as f64) - constant).powf(2.0) / constant;
    }
    log::debug!("{}: Value of chi_square: {}", TEST_NAME, chi_square);

    // finally compute p-value with the incomplete gamma function: igamc(K/2, chi_square/2)
    let p_value = statrs::function::gamma::gamma_ur(
        ((config.pi_values.len() as f64) - 1.0) * 0.5,
        chi_square * 0.5,
    );
    log::info!("{}: p-value = {}", TEST_NAME, p_value);

    // capture the current time after the test got executed and calculate elapsed time
    let end_time = std::time::Instant::now();
    let elapsed_time = end_time.duration_since(start_time).as_secs_f64();
    log::info!("{} took {:.6} seconds", TEST_NAME, elapsed_time);

    Ok(p_value)
}

/// Evaluate bit string length and select configuration parameters based on it.
///
/// # Arguments
///
/// length - Bit string length
///
/// # Return
///
/// Ok(config) - Config parameters based on bit string size
/// Err(err) - Some error occured
fn get_longest_run_config(length: usize) -> Result<customtypes::LongestRunConfig<'static>> {
    log::trace!("longest_run::get_longest_run_config()");

    // it is crucial to have at least 128 bit passed for the test
    if length < constants::MIN_LENGTH {
        anyhow::bail!(
            "{}: Bit string needs at least {} bits! Actual length: {}",
            TEST_NAME,
            constants::MIN_LENGTH,
            length
        );
    }

    // depending on length of bit string, choose the correct value for M (number of bits per
    // block), N (number of blocks), thresholds (min, max) and the pre-computed pi_values
    let config: customtypes::LongestRunConfig;

    if (constants::MIN_LENGTH..constants::MID_LENGTH).contains(&length) {
        config = customtypes::LongestRunConfig::create(
            constants::MIN_SIZE_M,
            constants::MIN_SIZE_N,
            constants::MIN_THRESHOLDS,
            &constants::MIN_PI_VALUES,
        );
    } else if (constants::MID_LENGTH..constants::MAX_LENGTH).contains(&length) {
        config = customtypes::LongestRunConfig::create(
            constants::MID_SIZE_M,
            constants::MID_SIZE_N,
            constants::MID_THRESHOLDS,
            &constants::MID_PI_VALUES,
        );
    } else {
        config = customtypes::LongestRunConfig::create(
            constants::MAX_SIZE_M,
            constants::MAX_SIZE_N,
            constants::MAX_THRESHOLDS,
            &constants::MAX_PI_VALUES,
        );
    }
    log::debug!("{}: Configured following values: {:?}", TEST_NAME, config);

    Ok(config)
}

/// Get the longest run of ones in a given block.
///
/// # Arguments
///
/// block - The block the longest run has to be computed from
///
/// # Return
///
/// max_count - Longest run number
fn count_max_consecutive_ones(block: &str) -> i32 {
    log::trace!("longest_run::count_max_consecutive_ones()");

    let mut max_count = 0;
    let mut current_count = 0;

    for bit in block.chars() {
        if bit == '1' {
            current_count += 1;
            max_count = max_count.max(current_count);
        } else {
            current_count = 0
        }
    }

    log::trace!(
        "{}: Block '{}', longest run of ones: {}",
        TEST_NAME,
        block,
        max_count
    );
    max_count
}

/// Calculcate the v_i values. Those are basically counters which longest run number occured how
/// often.
///
/// # Arguments
///
/// run_counts - A hashmap with collected longest run counts
/// thresholds - Minimum and maximum thresholds to merge specific counts
///
/// # Return
///
/// vi_counts - The collected v_i values
fn calculate_vi_values(
    run_counts: BTreeMap<i32, i32>,
    thresholds: (i32, i32),
) -> BTreeMap<i32, i32> {
    log::trace!("longest_run::calculate_vi_values()");

    let mut vi_counts: BTreeMap<i32, i32> = BTreeMap::new();

    for (&key, &value) in &run_counts {
        if key <= thresholds.0 {
            *vi_counts.entry(thresholds.0).or_insert(0) += value;
        } else if key >= thresholds.1 {
            *vi_counts.entry(thresholds.1).or_insert(0) += value;
        } else {
            *vi_counts.entry(key).or_insert(0) += value;
        }
    }

    // Iterate from the minimum threshold to the maximum threshold
    for threshold in thresholds.0..=thresholds.1 {
        // If there were no counts for the current threshold, insert a zero count for the current
        // threshold
        vi_counts.entry(threshold).or_insert(0);
    }

    vi_counts
}
