use rubato::{FastFixedIn, Resampler as RubatoResampler};

pub struct Resampler<T: rubato::Sample> {
    inner: FastFixedIn<T>,
    buffered_pcm: Vec<T>,
    channels: usize,
}
impl<T: rubato::Sample> Resampler<T> {
    pub fn new(
        in_sample_rate: usize,
        out_sample_rate: usize,
        channels: usize,
    ) -> anyhow::Result<Self> {
        let resample_ratio = out_sample_rate as f64 / in_sample_rate as f64;
        Ok(Self {
            inner: rubato::FastFixedIn::new(
                resample_ratio,
                10.,
                rubato::PolynomialDegree::Septic,
                1024,
                channels,
            )?,
            buffered_pcm: Vec::new(),
            channels,
        })
    }
    pub fn process(&mut self, data: &[T]) -> Vec<T> {
        self.buffered_pcm.extend_from_slice(data);
        let mut resampled_pcm = Vec::new();

        let chunk_size = 1024 * 2;
        let full_chunks = self.buffered_pcm.len() / chunk_size;
        let remainder = self.buffered_pcm.len() % chunk_size;
        for chunk in 0..full_chunks {
            let buffered_pcm = &self.buffered_pcm[chunk * chunk_size..(chunk + 1) * chunk_size];
            let d = deinterleave_audio(&buffered_pcm, self.channels);

            let pcm = self.inner.process(&d, None).unwrap();
            resampled_pcm.extend_from_slice(&interleave_audio(&pcm));
        }
        if remainder == 0 {
            self.buffered_pcm.clear();
        } else {
            self.buffered_pcm.copy_within(full_chunks * chunk_size.., 0);
            self.buffered_pcm.truncate(remainder);
        }
        resampled_pcm
    }
}
pub fn deinterleave_audio<T: rubato::Sample>(
    interleaved_data: &[T],
    num_channels: usize,
) -> Vec<Vec<T>> {
    let samples_per_channel = interleaved_data.len() / num_channels;

    // Create a vector to hold each channel's data
    let mut channel_data: Vec<Vec<T>> = vec![Vec::with_capacity(samples_per_channel); num_channels];

    // Distribute the interleaved samples to their respective channels
    for (i, &sample) in interleaved_data.iter().enumerate() {
        let channel_idx = i % num_channels;
        channel_data[channel_idx].push(sample);
    }

    channel_data
}
pub fn interleave_audio<T: rubato::Sample>(channel_data: &[Vec<T>]) -> Vec<T> {
    if channel_data.is_empty() {
        return Vec::new();
    }

    let num_channels = channel_data.len();
    let samples_per_channel = channel_data[0].len();

    // Verify all channels have the same number of samples
    for channel in channel_data.iter() {
        if channel.len() != samples_per_channel {
            panic!("All channels must have the same number of samples");
        }
    }

    // Create the output buffer with enough space for all samples
    let mut interleaved = Vec::with_capacity(num_channels * samples_per_channel);

    // Interleave the samples
    for sample_idx in 0..samples_per_channel {
        for channel_idx in 0..num_channels {
            interleaved.push(channel_data[channel_idx][sample_idx]);
        }
    }

    interleaved
}
