use nannou_osc as osc;
use nih_plug::prelude::*;
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
    #[nested(array, group = "Array Parameters")]
    pub array_params: Vec<ArrayParams>,
}

#[derive(Params)]
struct ArrayParams {
    /// This parameter's ID will get a `_1`, `_2`, and a `_3` suffix because of how it's used in
    /// `array_params` above.
    #[id = "channel"]
    pub val: FloatParam,
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
        let port = 9009;
        let target_addr = format!("{}:{}", "127.0.0.1", port);

        Self {
            array_params: (1..65)
                .map(|index| {
                    let sender = osc::sender()
                        .expect("Could not bind to default socket")
                        .connect(target_addr.clone())
                        .expect("Could not connect to socket at address");

                    ArrayParams {
                        val: FloatParam::new(
                            format!("Ch. {index}"),
                            0.0,
                            FloatRange::Linear { min: 0.0, max: 1.0 },
                        )
                        .with_callback(Arc::new(move |v| {
                            sender.send((
                                format!("/{}", index).to_string(),
                                vec![osc::Type::Float(v)],
                            ));
                        })),
                    }
                })
                .collect::<Vec<ArrayParams>>(),
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
    const DEFAULT_OUTPUT_CHANNELS: u32 = 1;

    const DEFAULT_AUX_INPUTS: Option<AuxiliaryIOConfig> = None;
    const DEFAULT_AUX_OUTPUTS: Option<AuxiliaryIOConfig> = None;

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    
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

    // fn accepts_bus_config(&self, config: &BusConfig) -> bool {
    //     // This works with any symmetrical IO layout
    //     config.num_input_channels == config.num_output_channels && config.num_input_channels > 0
    // }

    // This plugin doesn't need any special initialization, but if you need to do anything expensive
    // then this would be the place. State is kept around when the host reconfigures the
    // plugin. If we do need special initialization, we could implement the `initialize()` and/or
    // `reset()` methods
    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // for channel_samples in buffer.iter_samples() {
        //     // Smoothing is optionally built into the parameters themselves
        //     let gain = self.params.gain.smoothed.next();

        //     for sample in channel_samples {
        //         *sample *= gain;
        //     }
        // }

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
        ClapFeature::Utility,
        ClapFeature::Instrument,
    ];
}

impl Vst3Plugin for SpaceRadio {
    const VST3_CLASS_ID: [u8; 16] = *b"spacebroadc4stor";
    const VST3_CATEGORIES: &'static str = "Tools|Utilities";
}

nih_export_clap!(SpaceRadio);
nih_export_vst3!(SpaceRadio);
