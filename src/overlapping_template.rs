//! This module performs the Overlapping Template Matching Test.
//!
//! Description of test from NIST SP 800-22:
//!
//! "The focus of the Overlapping Template Matching test is the number of occurrences of pre-specified target
//! strings. Both this test and the Non-overlapping Template Matching test of Section 2.7 use an m-bit
//! window to search for a specific m-bit pattern. As with the test in Section 2.7, if the pattern is not found,
//! the window slides one bit position. The difference between this test and the test in Section 2.7 is that
//! when the pattern is found, the window slides only one bit before resuming the search."

use crate::constants;
use crate::customtypes;
use crate::utils;
use anyhow::{Context, Result};

const TEST_NAME: customtypes::Test = customtypes::Test::OverlappingTemplate;

/// Perform the Overlapping Template Matching Test by determining the p-value.
///
/// # Arguments
///
/// bit_string - The bit string to be tested for randomness
/// template_len - Length of templates to be used for test
/// number_of_blocks - The number of blocks the bit string has to be divided into
///
/// # Return
///
/// Ok(p-value) - The p-value which indicates whether randomness is given or not
/// Err(err) - Some error occured
pub fn perform_test(bit_string: &str, template_len: usize, number_of_blocks: usize) -> Result<f64> {
    log::trace!("overlapping_template::perform_test()");

    // capture the current time before executing the actual test
    let start_time = std::time::Instant::now();

    // check if bit string contains invalid characters
    let length = utils::evaluate_bit_string(
        TEST_NAME,
        bit_string,
        constants::RECOMMENDED_SIZE_OVERLAPPING_TEMPLATE,
    )
    .with_context(|| "Invalid character(s) in passed bit string detected")?;

    // evaluate the other input and get the block size m
    let block_size = evaluate_test_params(length, template_len, number_of_blocks)
        .with_context(|| "Template length does not match defined requirements")?;

    // calculate number of templates to be searched
    let number_of_templates = 2_usize.pow(template_len.try_into().unwrap());

    // calculate theoretical mean and variance
    let first_fraction = 1.0 / (number_of_templates as f64);
    let second_fraction =
        (2.0 * (template_len as f64) - 1.0) / 2.0_f64.powf(2.0 * (template_len as f64));

    let mean = ((block_size - template_len + 1) as f64) / (number_of_templates as f64);
    let variance = (block_size as f64) * (first_fraction - second_fraction);
    log::debug!(
        "{}: Theoretical mean = {}, Variance = {}",
        TEST_NAME,
        mean,
        variance
    );

    // now iterate over each template and search for it in each substring
    let mut p_values = Vec::<f64>::new();

    for num in 0..number_of_templates {
        let template = format!("{:0width$b}", num, width = template_len);
        let mut template_counters = Vec::<usize>::new();

        // now iterate over blocks 1...N and count occurences of respective template in substring
        for block in 0..number_of_blocks {
            let start_index = block * block_size;
            let end_index = (block + 1) * block_size;
            let substring = &bit_string[start_index..end_index];

            let mut counter = 0;
            let mut index = 0;

            while let Some(start) = substring[index..].find(&template) {
                counter += 1;

                // move the index to the next possible occurence
                index += start + template_len;
            }

            log::trace!(
                "{}: Template '{}' in substring '{}' found {} times",
                TEST_NAME,
                template,
                substring,
                counter
            );
            template_counters.push(counter);
        }
        // compute chi_square statistics
        let mut chi_square = 0.0;
        for counter in &template_counters {
            chi_square += ((*counter as f64) - mean).powf(2.0) / variance;
        }
        log::trace!(
            "{}: Chi_square = {} for template '{}'",
            TEST_NAME,
            chi_square,
            template
        );

        // now compute p-value for current template with incomplete gamma function
        let p_value = if chi_square == 0.0 {
            1.0
        } else {
            statrs::function::gamma::gamma_ur((number_of_blocks as f64) * 0.5, chi_square * 0.5)
        };
        log::trace!(
            "{}: p-value = {} for template '{}'",
            TEST_NAME,
            p_value,
            template
        );

        p_values.push(p_value);
    }

    let p_values_mean = p_values.iter().sum::<f64>() / (p_values.len() as f64);
    log::info!("{}: Mean of p-values = {}", TEST_NAME, p_values_mean);

    // capture the current time after the test got executed and calculate elapsed time
    let end_time = std::time::Instant::now();
    let elapsed_time = end_time.duration_since(start_time).as_secs_f64();
    log::info!("{} took {:.6} seconds", TEST_NAME, elapsed_time);

    Ok(p_values_mean)
}

/// Evaluate passed test parameters and return the resulting block size M.
///
/// # Arguments
///
/// bit_string_length - Length of bit string
/// template_len - Length of template to be searched later in substrings
/// number_of_blocks - The number of blocks the bitstring has to be divided into
///
/// # Return
///
/// Ok(block_size) - The resulting block size if template length is okay
/// Err(err) - Some error occured
fn evaluate_test_params(
    bit_string_length: usize,
    template_len: usize,
    number_of_blocks: usize,
) -> Result<usize> {
    log::trace!("non_overlapping_template::evaluate_test_params()");

    // check whether template length is between thresholds for meaningful results
    if !(constants::TEMPLATE_LEN.0..constants::TEMPLATE_LEN.1 + 1).contains(&template_len) {
        anyhow::bail!(
            "{}: Passed template length '{}' must be between {} and {}",
            TEST_NAME,
            template_len,
            constants::TEMPLATE_LEN.0,
            constants::TEMPLATE_LEN.1
        );
    }

    // recommended sizes for template lengths: 9, 10. Log a warning if they do not match
    if template_len < constants::TEMPLATE_LEN.1 - 1 {
        log::warn!(
            "{}: Recommended size for template length: {}, {}",
            TEST_NAME,
            constants::TEMPLATE_LEN.1 - 1,
            constants::TEMPLATE_LEN.1
        );
    }

    // check number of blocks
    if number_of_blocks > constants::RECOMMENDED_SIZE {
        anyhow::bail!(
            "{}: Number of blocks N ({}) is greater than recommended size ({})",
            TEST_NAME,
            number_of_blocks,
            constants::RECOMMENDED_SIZE
        );
    }

    // construct block size M to get the substrings to be tested
    let block_size = bit_string_length / number_of_blocks;
    let recommended_size = bit_string_length / 100;

    if block_size <= recommended_size {
        anyhow::bail!(
            "{}: Block size M ({}) is less than or equal to {}. Choose smaller number of blocks",
            TEST_NAME,
            block_size,
            recommended_size
        );
    }

    log::info!(
        "{}: Template length = {}, Block size M = {}, Number of blocks N = {}",
        TEST_NAME,
        template_len,
        block_size,
        number_of_blocks
    );

    Ok(block_size)
}
