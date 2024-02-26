//! This module performs the Frequency Monobit Test.
//! If this test does not pass, the remaining tests are NOT executed (makes sense, right?)
//!
//! Description of test from NIST SP 800-22:
//!
//! "The focus of the test is the proportion of zeroes and ones for the entire sequence. The purpose of this test
//! is to determine whether the number of ones and zeros in a sequence are approximately the same as would
//! be expected for a truly random sequence. The test assesses the closeness of the fraction of ones to 1⁄2, that
//! is, the number of ones and zeroes in a sequence should be about the same. All subsequent tests depend on
//! the passing of this test."

use anyhow::Result;

/// Perform the Frequency Monobit Test by determining the p-value.
///
/// # Arguments
///
/// bit_string - The bit string to be tested for randomness
///
/// # Return
///
/// Ok(p-value) - The p-value which indicates whether randomness is given or not
/// Err(err) - Some error occured
pub fn perform_test(bit_string: &str) -> Result<f64> {
    log::trace!("frequency_monobit::perform_test()");

    // check validity of passed bit string
    if bit_string.is_empty() || bit_string.chars().any(|c| c != '0' && c != '1') {
        anyhow::bail!("Bit string is either empty or contains invalid character(s)");
    }

    let length = bit_string.len();
    log::debug!("Bit string has the length {}", length);

    // Recommended size is at least 100 bits. It is not an error but log a warning anyways
    if length < 100 {
        log::warn!(
            "Recommended size is at least 100 bits. Consider imprecision when calculating p-value"
        );
    }

    // first of all, we need to compute the partial sum S_n. This is the difference between #ones and #zeroes
    let count_zero = bit_string.chars().filter(|&c| c == '0').count();
    let count_one = length - count_zero;

    log::info!(
        "Bit string contains {} zeros and {} ones",
        count_zero,
        count_one
    );

    let partial_sum = if count_zero >= count_one {
        (count_zero - count_one) as f64
    } else {
        (count_one - count_zero) as f64
    };

    // now calculate observed value S_obs = |S_n| / sqrt(length)
    let observed = partial_sum / (length as f64).sqrt();
    log::debug!("Observed value S_obs: {}", observed);

    // finally, compute p-value to decide whether given bit string is random or not
    // Therefore we need the complementary error function: erfc(observed / sqrt(2))
    let p_value = statrs::function::erf::erfc(observed / f64::sqrt(2.0));
    log::info!("Frequency Monobit: p-value of bit string is {}", p_value);

    Ok(p_value)
}
