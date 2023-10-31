use dashmap::DashSet;
use nannou_osc as osc;
use nih_plug::prelude::*;
use osc::Sender;
use std::{
    sync::{mpsc, Arc, Mutex, RwLock},
    thread,
};

struct SpaceRadio {
    params: Arc<SpaceRadioParams>,
    sender: Arc<Mutex<Option<Sender>>>,
    dirty_params: Arc<DashSet<usize>>,
}

/// The [`Params`] derive macro gathers all of the information needed for the wrapper to know about
/// the plugin's parameters, persistent serializable fields, and nested parameter groups. You can
/// also easily implement [`Params`] by hand if you want to, for instance, have multiple instances
/// of a parameters struct for multiple identical oscillators/filters/envelopes.
#[derive(Params)]
struct SpaceRadioParams {
    #[nested(array, group = "Array Parameters")]
    pub array_params: Vec<ArrayParams>,
    #[persist = "osc_address"]
    osc_destination_address: RwLock<String>,
    #[persist = "osc_port"]
    osc_destination_port: RwLock<u16>,
}

#[derive(Params)]
struct ArrayParams {
    /// This parameter's ID will get a `_1`, `_2`, and a `_3` suffix because of how it's used in
    /// `array_params` above.
    #[id = "channel"]
    pub val: FloatParam,
}

impl SpaceRadio {
    fn setup_sender(&mut self) {
        let (tx_sender, rx_sender) = mpsc::channel();

        thread::spawn(move || {
            let sender = Arc::new(Mutex::new(Some(
                osc::sender().expect("Could not bind to default socket"), // .connect(target_addr.clone())
                                                                          // .expect("Could not connect to socket at address"),
            )));

            tx_sender.send(sender).unwrap();
        });

        let sender = rx_sender.recv().unwrap();
        self.sender = sender;
    }
}

impl Default for SpaceRadio {
    fn default() -> Self {
        let (tx_dirty_params, rx_dirty_params) = mpsc::channel();
        thread::spawn(move || {
            tx_dirty_params
                .send(Arc::new(DashSet::<usize>::new()))
                .unwrap();
        });
        let dirty_params = rx_dirty_params.recv().unwrap();

        let mut space_radio = Self {
            params: Arc::new(SpaceRadioParams::new(&dirty_params)),
            sender: Arc::new(Mutex::new(None)),
            dirty_params,
        };

        space_radio.setup_sender();
        space_radio
    }
}

impl SpaceRadioParams {
    fn new(dirty_params: &Arc<DashSet<usize>>) -> Self {
        Self {
            array_params: (0..64)
                .map(|index| {
                    let dirty_params = Arc::clone(dirty_params);
                    ArrayParams {
                        val: FloatParam::new(
                            format!("Ch. {index}", index = index + 1),
                            0.0,
                            FloatRange::Linear { min: 0.0, max: 1.0 },
                        )
                        .with_callback(Arc::new(move |_| {
                            dirty_params.as_ref().insert(index);
                        })),
                    }
                })
                .collect::<Vec<ArrayParams>>(),
            osc_destination_address: RwLock::new("127.0.0.1".into()),
            osc_destination_port: RwLock::new(9009),
        }
    }
}

enum BackgroundTask {
    UpdateParameter { index: usize, value: f32 },
    // SetupSender,
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
    type BackgroundTask = BackgroundTask;

    fn task_executor(&self) -> TaskExecutor<Self> {
        let sender = Arc::clone(&self.sender);
        let port = *self.params.osc_destination_port.read().unwrap();
        let osc_destination_address = self.params.osc_destination_address.read().unwrap().clone();

        Box::new(move |task| match task {
            BackgroundTask::UpdateParameter { index, value } => {
                let sender = sender.lock().unwrap();
                let target_addr = format!("{osc_destination_address}:{port}");

                match sender.as_ref() {
                    None => {
                        // println!("No sender");
                    }
                    Some(sender) => {
                        let addr = format!("/{index}").to_string();
                        let value = vec![osc::Type::Float(value)];
                        // println!("Sent {index} {value:?}");
                        sender
                            .send((addr, value), target_addr)
                            .expect("Could not send message");
                    }
                }
            }
        })
    }

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
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for index in self.dirty_params.iter() {
            let value = self.params.array_params[*index].val.value();
            context.execute_background(BackgroundTask::UpdateParameter {
                index: *index,
                value,
            });
        }

        self.dirty_params.clear();

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
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility, ClapFeature::Instrument];
}

impl Vst3Plugin for SpaceRadio {
    const VST3_CLASS_ID: [u8; 16] = *b"spacebroadc4stor";
    const VST3_CATEGORIES: &'static str = "Tools|Utilities";
}

nih_export_clap!(SpaceRadio);
nih_export_vst3!(SpaceRadio);
