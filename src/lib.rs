use nih_plug::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;

struct SpaceRadio {
    params: Arc<SpaceRadioParams>,
}

/// The [`Params`] derive macro gathers all of the information needed for the wrapper to know about
/// the plugin's parameters, persistent serializable fields, and nested parameter groups. You can
/// also easily implement [`Params`] by hand if you want to, for instance, have multiple instances
/// of a parameters struct for multiple identical oscillators/filters/envelopes.
#[derive(Params)]
struct SpaceRadioParams {
    /// The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

    /// This field isn't used in this example, but anything written to the vector would be restored
    /// together with a preset/state file saved for this plugin. This can be useful for storing
    /// things like sample data.
    #[persist = "industry_secrets"]
    pub random_data: Mutex<Vec<f32>>,

    /// You can also nest parameter structs. These will appear as a separate nested group if your
    /// DAW displays parameters in a tree structure.
    #[nested(group = "Subparameters")]
    pub sub_params: SubParams,

    /// Nested parameters also support some advanced functionality for reusing the same parameter
    /// struct multiple times.
    #[nested(array, group = "Array Parameters")]
    pub array_params: Vec<ArrayParams>,
}

#[derive(Params)]
struct SubParams {
    #[id = "thing"]
    pub nested_parameter: FloatParam,
}

#[derive(Params)]
struct ArrayParams {
    /// This parameter's ID will get a `_1`, `_2`, and a `_3` suffix because of how it's used in
    /// `array_params` above.
    #[id = "lol"]
    pub nope: FloatParam,
}

impl Default for SpaceRadio {
    fn default() -> Self {
        Self {
            params: Arc::new(SpaceRadioParams::default()),
        }
    }
}

impl Default for SpaceRadioParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "SpaceRadio",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            // Persisted fields can be initialized like any other fields, and they'll keep their
            // values when restoring the plugin's state.
            random_data: Mutex::new(Vec::new()),
            // sub_params: SubParams {
            //     nested_parameter: FloatParam::new(
            //         "Unused Nested Parameter",
            //         0.5,
            //         FloatRange::Skewed {
            //             min: 2.0,
            //             max: 2.4,
            //             factor: FloatRange::skew_factor(2.0),
            //         },
            //     )
            //     .with_value_to_string(formatters::v2s_f32_rounded(2)),
            // },
            array_params: (1..16).map(|index| ArrayParams {
                nope: FloatParam::new(
                    format!("Channel {index}"),
                    0.0,
                    FloatRange::Linear { min: 0, max: 1.0 },
                ),
            }).collect::<Vec<ArrayParams>>(),
        }
    }
}

impl Plugin for SpaceRadio {
    const NAME: &'static str = "Space Radio";
    const VENDOR: &'static str = "@micahscopes";
    // You can use `env!("CARGO_PKG_HOMEPAGE")` to reference the homepage field from the
    // `Cargo.toml` file here
    const URL: &'static str = "https://wondering.xyz";
    const EMAIL: &'static str = "micahscopes@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const DEFAULT_INPUT_CHANNELS: u32 = 0;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 0;

    const DEFAULT_AUX_INPUTS: Option<AuxiliaryIOConfig> = None;
    const DEFAULT_AUX_OUTPUTS: Option<AuxiliaryIOConfig> = None;

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    // Setting this to `true` will tell the wrapper to split the buffer up into smaller blocks
    // whenever there are inter-buffer parameter changes. This way no changes to the plugin are
    // required to support sample accurate automation and the wrapper handles all of the boring
    // stuff like making sure transport and other timing information stays consistent between the
    // splits.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This works with any symmetrical IO layout
        config.num_input_channels == config.num_output_channels && config.num_input_channels > 0
    }

    // This plugin doesn't need any special initialization, but if you need to do anything expensive
    // then this would be the place. State is kept around when the host reconfigures the
    // plugin. If we do need special initialization, we could implement the `initialize()` and/or
    // `reset()` methods

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            for sample in channel_samples {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }

    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
    fn deactivate(&mut self) {}
}

impl ClapPlugin for SpaceRadio {
    const CLAP_ID: &'static str = "xyz.wondering.space-radio";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("OSC broadcaster");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::MultiEffects,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for SpaceRadio {
    const VST3_CLASS_ID: [u8; 16] = *b"spacebroadc4stor";
    const VST3_CATEGORIES: &'static str = "Tools|Utilities";
}

nih_export_clap!(SpaceRadio);
nih_export_vst3!(SpaceRadio);