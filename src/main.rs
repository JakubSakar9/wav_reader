use std::fs::File;
use std::path::Path;
use std::io::Error;
use std::io::prelude::*;
use std::process::exit;

use wav::header::Header;
use wav::bit_depth::BitDepth;

struct DataChunk {
    irg_length: u16,
    data: Vec<u8>
}

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

fn analyze_signal(data: Vec<f32>, threshold: f32) -> (Vec<u32>, Vec<u32>) {
    let num_samples: u32 = data.len() as u32;
    let mut num_pulses: u32 = 0;
    let mut last_max_idx: u32 = 0;
    let mut last_max: f32 = 0.0;
    let mut last_sample: f32 = 2.0;
    let mut periods: Vec<u32> = vec![];
    let mut cum_samples: Vec<u32> = vec![];
    println!("Measuring period lengths in the signal...");
    for i in 0..(num_samples - 1) {
        let x: f32 = data[i as usize];
        if i == 0 {
            last_max_idx = 0;
            last_max = x;
        }

        if x > last_sample {
            let pulse_length: u32 = i - last_max_idx;
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
            periods.push(pulse_length as u32);
            cum_samples.push(i);
        }

        last_sample = x;
    }
    println!("Number of pulses in the signal: {}", num_pulses);
    return (periods, cum_samples);
}

fn compute_threshold(periods: &Vec<u32>) -> f32 {
    let mut periods_s = periods.to_vec();
    periods_s.sort();
    let octile: usize = periods.len() >> 3;
    (periods_s[octile] + periods_s[periods.len() - octile]) as f32 / 2.0
}

fn measure_irg(pulses: Vec<u8>, sample_rate: u32, cum_samples: Vec<u32>) -> Vec<u16> {
    let irg_threshold = 100;
    let mut cur_streak: u32 = 0;
    let mut last_streak_start: i32 = -1;
    let mut result: Vec<u16> = vec![];

    for i in 0..pulses.len() {
        let cur_value: u16 = pulses[i] as u16;
        if cur_value == 1 {
            cur_streak += 1;
        }
        else {
            if cur_streak > irg_threshold {
                let cur_streak_samples: u32;
                if last_streak_start < 0 {
                    cur_streak_samples = cum_samples[i];
                }
                else {
                    cur_streak_samples = cum_samples[i] - cum_samples[last_streak_start as usize];
                }
                let irg_ms: f32 = cur_streak_samples as f32 * 1000.0 / sample_rate as f32;
                result.push(irg_ms as u16);
            }
            cur_streak = 0;
            last_streak_start = i as i32;
        }
        result.push(cur_value);
    }
    return result;
}

fn write_collapsed_data(bit_value: u16, cluster_length: u32, data_vec: &mut Vec<u16>) {
    if bit_value > 1 {
        data_vec.push(bit_value);
        return;
    }
    let singular_cluster_length: f32 = if bit_value == 0 {6.5} else {8.45};
    let num_bits_f: f32 = cluster_length as f32  / singular_cluster_length;
    let num_bits = (num_bits_f + 0.5) as i32;
    if num_bits > 10 {
        return;
    }
    for _ in 0..num_bits {
        data_vec.push(bit_value);
    }
}

fn normal_to_raw_binary(periods: &Vec<u32>, threshold: f32, sample_rate: u32, cum_samples: Vec<u32>) -> Vec<u16> {
    let pulses: Vec<u8> = periods.iter().map(|a| if *a as f32 > threshold {0} else {1}).collect();
    let pulses_irg: Vec<u16> = measure_irg(pulses, sample_rate, cum_samples);

    let mut result: Vec<u16> = vec![];
    let mut cluster_length: u32 = 1;
    let mut last_bit: u16 = 1;
    for i in 0..pulses_irg.len() {
        let x: u16 = pulses_irg[i];
        if i == 0 {
            last_bit = x;
            continue;
        }
        else if last_bit != 0 && x == 0 {
            write_collapsed_data(1, cluster_length, &mut result);
            cluster_length = 0;
        }
        else if last_bit != 1 && x == 1 {
            write_collapsed_data(0, cluster_length, &mut result);
            cluster_length = 0;
        }
        else if i == pulses_irg.len() - 1 {
            write_collapsed_data(x, cluster_length + 1, &mut result);
        }
        else {
            write_collapsed_data(x, 1, &mut result);
        }
        
        last_bit = x;
        cluster_length += 1;
    }
    for i in 0..256 {
        print!("{}", result[i]);
    }
    return result;
}

fn extract_data(input: Vec<u16>) -> Vec<DataChunk> {
    let mut output: Vec<DataChunk> = Vec::new();
    let mut chunk_data: Vec<u8> = vec![];
    let mut bit_counter = 0; 
    let mut byte_counter: u16 = 0;
    let mut byte_buffer: Vec<u8> = vec![];
    let mut in_data = false;
    let mut cur_irg: u16 = 0;

    let data_length_bytes: u16 = 132;

    for &bit in input.iter() {
        if  byte_counter == data_length_bytes {
            let mut data_chunk: DataChunk = DataChunk { irg_length: 0, data: vec![]};
            data_chunk.irg_length = cur_irg;
            data_chunk.data = chunk_data.clone();
            output.push(data_chunk);
            byte_counter = 0;
            in_data = false;
        }
        if in_data {
            if bit_counter != 0 && bit_counter != 9 {
                byte_buffer.push(bit as u8);
            }
            bit_counter += 1;
            if bit_counter == 10 {   
                bit_counter = 0;
                byte_counter += 1;
                byte_buffer.reverse();
                chunk_data.append(&mut byte_buffer);
            }
        } else if bit > 1 {
            in_data = true;
            cur_irg = bit;
            // print!("{cur_irg} ");
        }        
    }
    output
}

fn write_to_binary(data: &Vec<DataChunk>) -> std::io::Result<()> {
    println!("Writing to binary...");
    let mut file = File::create("out/out.bin")?;

    for chunk in data {
        let mut byte_counter = 0;
        let mut byte: u8 = 0;

        let chunk_data: &Vec<u8> = &chunk.data;
        let irg_length: u16 = chunk.irg_length;
        file.write_all(&[(irg_length - irg_length << 8) as u8, (irg_length << 8) as u8])?;
        for &bit in chunk_data.iter() {
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
    }

    Ok(())
}

fn write_cas_fuji(out_file: &mut File) -> std::io::Result<()> {
    // The FUJI literal
    out_file.write_all(&[0x46, 0x55, 0x4a, 0x49])?;
    // Header metadata for empty FUJI chunk
    out_file.write_all(&[0x00, 0x00, 0x00, 0x00])?;
    Ok(())
}

fn write_cas_baud(out_file: &mut File, baud_rate: u16) -> std::io::Result<()> {
    // The baud literal
    out_file.write_all(&[0x62, 0x61, 0x75, 0x64])?;
    // Zero chunk length
    out_file.write_all(&[0x00, 0x00])?;
    // Write baud rate
    out_file.write_all(&[(baud_rate - baud_rate << 8) as u8, (baud_rate << 8) as u8])?;
    Ok(())
}

fn write_cas_data(data: &Vec<u8>, irg_len: u16, out_file: &mut File) -> std::io::Result<()> {
    let num_bytes: u16 = data.len() as u16;
    // The data literal
    out_file.write_all(&[0x64, 0x61, 0x74, 0x61])?;
    // Chunk length
    out_file.write_all(&[(num_bytes - num_bytes << 8) as u8, (num_bytes << 8) as u8])?;
    // IRG length
    out_file.write_all(&[(irg_len - irg_len << 8) as u8, (irg_len << 8) as u8])?;

    // Bytes
    for &byte in data.iter() {
        out_file.write_all(&[byte])?;
    }
    Ok(())
}

fn write_to_cas(data: &Vec<u8>) -> std::io::Result<()> {
    println!("Writing to cas...");
    let mut file = File::create("out/out.cas")?;
    Ok(())
}

fn process_wav(header: Header, raw_data: BitDepth) {
    print_stats(&header);
    let data: Vec<f32> = to_float_vec(raw_data);
    let (periods, cum_samples): (Vec<u32>, Vec<u32>) = analyze_signal(data, 0.3);
    let threshold: f32 = compute_threshold(&periods);
    let binary: Vec<u16> = normal_to_raw_binary(&periods, threshold, header.sampling_rate, cum_samples);
    let binary_cut: Vec<DataChunk> = extract_data(binary);
    let data_size: usize = binary_cut.len();
    match write_to_binary(&binary_cut) {
        Ok(_) => {println!("Successfully written {data_size} chunks to file!");}
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
