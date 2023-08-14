use std::fs::File;
use std::path::Path;
use std::io::Error;
use std::io::prelude::*;

use wav::header::Header;
use wav::bit_depth::BitDepth;

fn read_wav_data(filepath: &str) -> Result<(Header, BitDepth), Error> {
    println!("Reading wav data...");
    let mut inp_file = File::open(Path::new(filepath))?;
    let (header, data) = wav::read(&mut inp_file)?;
    return Ok((header, data))
}

fn print_stats(header: &Header) {
    println!("Channel count: {}", header.channel_count);
    println!("Sampling rate: {}Hz", header.sampling_rate);
}

fn to_float_vec(data: BitDepth) -> Vec<f32> {
    let result: Vec<f32>;
    match data {
        BitDepth::Eight(int_data) => {
            println!("8bit integer, needs conversion");
            println!("Finding the minimal value...");
            let min_val = *int_data.iter().min().unwrap_or(&0);
            println!("Minimal value: {min_val}");
            println!("Finding the maximal value...");
            let max_val = *int_data.iter().max().unwrap_or(&0);
            println!("Minimal value: {max_val}");
            let range_val = max_val as f32 - min_val as f32;

            println!("Normalizing to float...");
            result = int_data.iter()
                .map(|val| 2.0 * (*val as f32 - min_val as f32) / range_val as f32 - 1.0)
                .collect();
            println!("Normalized!");
            result
        },
        BitDepth::Sixteen(int_data) => {
            println!("16bit integer, needs conversion");
            println!("Finding the minimal value...");
            let min_val = *int_data.iter().min().unwrap_or(&0);
            println!("Minimal value: {min_val}");
            println!("Finding the maximal value...");
            let max_val = *int_data.iter().max().unwrap_or(&0);
            println!("Minimal value: {max_val}");
            let range_val = max_val as f32 - min_val as f32;

            println!("Normalizing to float...");
            result = int_data.iter()
                .map(|val| 2.0 * (*val as f32 - min_val as f32) / range_val as f32 - 1.0)
                .collect();
            println!("Normalized!");
            result
        },
        BitDepth::TwentyFour(int_data) => {
            println!("24bit integer, needs conversion");
            println!("Finding the minimal value...");
            let min_val = *int_data.iter().min().unwrap_or(&0);
            println!("Minimal value: {min_val}");
            println!("Finding the maximal value...");
            let max_val = *int_data.iter().max().unwrap_or(&0);
            println!("Maximal value: {max_val}");
            let range_val: f32 = max_val as f32 - min_val as f32;

            println!("Normalizing to float...");
            result = int_data.iter()
                .map(|val| 2.0 * (*val as f32 - min_val as f32) / range_val - 1.0)
                .collect();
            println!("Normalized!");
            result
        },
        BitDepth::ThirtyTwoFloat(float_data) => {
            println!("32bit float, no conversion required");
            float_data
        },
        BitDepth::Empty => {
            println!("Empty");
            vec![]
        }
    }
}

fn signal_to_periods(data: Vec<f32>, threshold: f32) -> Vec<u32> {
    let num_samples: u64 = data.len() as u64;
    let mut num_pulses: u32 = 0;
    let mut last_max_idx: u64 = 0;
    let mut last_max: f32 = 0.0;
    let mut last_sample: f32 = 2.0;
    let mut result: Vec<u32> = vec![];
    println!("Measuring period lengths in the signal...");
    for i in 0..(num_samples - 1) {
        let x: f32 = data[i as usize];
        if i == 0 {
            last_max_idx = 0;
            last_max = x;
        }

        if x > last_sample {
            let pulse_length: u64 = i - last_max_idx;
            last_max_idx = i;
            if pulse_length < 5 || pulse_length > 50 {
                last_sample = x;
                last_max = x;
                continue;
            }

            let sample_diff = last_max - x;
            last_max = x;
            if sample_diff < threshold {
                last_sample = x;
                continue;
            }

            num_pulses += 1;
            result.push(pulse_length as u32);
        }

        last_sample = x;
    }
    println!("Number of pulses in the signal: {}", num_pulses);
    return result;
}

fn compute_threshold(periods: &Vec<u32>) -> f32 {
    let mut periods_s = periods.to_vec();
    periods_s.sort();
    let octile: usize = periods.len() >> 3;
    (periods_s[octile] + periods_s[periods.len() - octile]) as f32 / 2.0
}

fn write_collapsed_data(bit_value: u8, cluster_length: u32, data_vec: &mut Vec<u8>) {
    let singular_cluster_length: f32 = if bit_value == 0 {6.5} else {8.45};
    let num_bits_f: f32 = cluster_length as f32  / singular_cluster_length;
    let num_bits = (num_bits_f + 0.5) as i32;
    for _ in 0..num_bits {
        data_vec.push(bit_value);
    }
}

fn normal_to_raw_binary(periods: &Vec<u32>, threshold: f32) -> Vec<u8> {
    let pulses: Vec<u8> = periods.iter().map(|a| if *a as f32 > threshold {0} else {1}).collect();

    let mut result: Vec<u8> = vec![];
    let mut cluster_length: u32 = 1;
    let mut last_bit: u8 = 1;
    for i in 0..pulses.len() {
        let x: u8 = pulses[i];
        if i == 0 {
            last_bit = x;
            continue;
        }
        else if last_bit == 1 && x == 0 {
            write_collapsed_data(1, cluster_length, &mut result);
            cluster_length = 0;
        }
        else if last_bit == 0 && x == 1 {
            write_collapsed_data(0, cluster_length, &mut result);
            cluster_length = 0;
        }
        else if i == pulses.len() - 1 {
            write_collapsed_data(x, cluster_length + 1, &mut result);
        }
        
        last_bit = x;
        cluster_length += 1;
    }
    return result;
}

fn extract_data(input: Vec<u8>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut bit_counter = 0; 
    let mut byte_counter: u16 = 0;
    let mut byte_buffer: Vec<u8> = vec![];
    let mut in_data = false;

    let data_length_bytes: u16 = 132;

    for &bit in input.iter() {
        if  byte_counter == data_length_bytes {
            byte_counter = 0;
            in_data = false;
        }
        if in_data {
            if bit_counter != 0 && bit_counter != 9 {
                byte_buffer.push(bit);
            }
            bit_counter += 1;
            if bit_counter == 10 {   
                bit_counter = 0;
                byte_counter += 1;
                byte_buffer.reverse();
                output.append(&mut byte_buffer);
            }
        } else if bit == 0 {
            bit_counter += 1;
            in_data = true;
        }        
    }
    output
}

fn write_to_file_text(data: &Vec<u8>) -> std::io::Result<()> {
    println!("Writing to file...");
    let mut file = File::create("out/bits_short.txt")?;

    for &bit in data.iter() {
        file.write_all(&[bit + 48])?;
    }
    Ok(())
}

fn write_to_file_bitwise(data: &Vec<u8>) -> std::io::Result<()> {
    println!("Writing to file...");
    let mut file = File::create("out/bytes.bin")?;
    let mut byte_counter = 0;
    let mut byte: u8 = 0;

    for &bit in data.iter() {
        byte <<= 1;
        byte |= bit;

        byte_counter += 1;
        if byte_counter == 8 {
            file.write_all(&[byte])?;
            byte_counter = 0;
            byte = 0;
        }
    }

    if byte_counter > 0 {
        byte <<= 8 - byte_counter;
        file.write_all(&[byte])?;
    }

    Ok(())
}

fn process_wav(header: Header, raw_data: BitDepth) {
    print_stats(&header);
    let data: Vec<f32> = to_float_vec(raw_data);
    let periods: Vec<u32> = signal_to_periods(data, 0.05);
    let threshold: f32 = compute_threshold(&periods);
    let binary: Vec<u8> = normal_to_raw_binary(&periods, threshold);
    let binary_cut: Vec<u8> = extract_data(binary);
    let data_size: usize = binary_cut.len();
    match write_to_file_bitwise(&binary_cut) {
        Ok(_) => {println!("Successfully written {data_size} bits to file!");}
        Err(_) => {print!("ERROR: Writing to file failed!");}
    }
}

fn main() {
    match read_wav_data("data/test.wav") {
        Err(_) => {println!("Failed to read audio data");},
        Ok((header, data)) => {
            process_wav(header, data);
        }
    }
}
