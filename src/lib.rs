#[macro_use]
extern crate vst;
extern crate time;
extern crate rand;
extern crate rand_distr;

use vst::buffer::AudioBuffer;
use vst::plugin::{Category, Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;
use vst::api;
use vst::buffer::{ SendEventBuffer};
use vst::event::{Event, MidiEvent};
use vst::plugin::{CanDo, HostCallback,};
use std::sync::Arc;
use rand::Rng;
use rand_distr::{Normal, Distribution};

/**
 * Parameters
 */ 
struct SlothParameters {
    variance: AtomicFloat,
}


impl Default for SlothParameters {
    fn default() -> SlothParameters {
        SlothParameters {
            variance: AtomicFloat::new(5.0),
        }
    }
}




/**
 * Plugin
 */ 
struct DelayedMidiEvent {
    event: MidiEvent,
    time_until_send: f32
}


#[derive(Default)]
struct Sloth {
    host: HostCallback,
    sample_rate: f32,
    immediate_events: Vec<MidiEvent>,
    delayed_events: Vec<DelayedMidiEvent>,
    send_buffer: SendEventBuffer,
    params: Arc<SlothParameters>,
}


impl Sloth {
    fn add_delayed_event(&mut self, e: MidiEvent) {
        let variance = self.params.variance.get() / 1000.0;
        
        let normal = Normal::new(0., variance).unwrap();
        let v = normal.sample(&mut rand::thread_rng()).abs() as f32;


        match e.data[0] {
            // only delay note-ons
            144 => self.delayed_events.push(DelayedMidiEvent {
                event: e,
                time_until_send: v
            }),

            _ => {
                self.immediate_events.push(e)
            }
        }
    }
    
    fn update_delayed_midi_events(&mut self) {
        // Delayed
        for mut delayed_event in &mut self.delayed_events {
            delayed_event.time_until_send -= 1.0 / self.sample_rate;
            
            // time to send
            if delayed_event.time_until_send <= 0.0 {
                self.send_buffer.send_events(vec![delayed_event.event], &mut self.host);                
            }
        }
        
        self.delayed_events.retain(|e| e.time_until_send > 0.0);

        // Immediate
        self.send_buffer.send_events(&self.immediate_events, &mut self.host);
        self.immediate_events.clear();
    }
}

impl Plugin for Sloth {
    fn new(host: HostCallback) -> Self {
        let mut p = Sloth::default();
        p.host = host;
        p.params = Arc::new(SlothParameters::default());
        p
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Sloth".to_string(),
            vendor: "Rein van der Woerd".to_string(),
            unique_id: 243723072,
            version: 1,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 1,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }

    fn process_events(&mut self, events: &api::Events) {
        for e in events.events() {
            #[allow(clippy::single_match)]
            match e {
                Event::Midi(e) => self.add_delayed_event(e),
                _ => (),
            }
        }
    }


    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        for (input, output) in buffer.zip() {
            for (in_sample, out_sample) in input.iter().zip(output) {
                *out_sample = *in_sample;
            }
        }
        self.update_delayed_midi_events();
    }

    fn can_do(&self, can_do: CanDo) -> vst::api::Supported {
        use vst::api::Supported::*;
        use vst::plugin::CanDo::*;

        match can_do {
            SendEvents | SendMidiEvent | ReceiveEvents | ReceiveMidiEvent => Yes,
            _ => No,
        }
    }


    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

impl PluginParameters for SlothParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.variance.get(),
            _ => 0.0,
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        #[allow(clippy::single_match)]
        match index {
            0 => self.variance.set(val.max(0.0000000001)),
            _ => (),
        }
    }

    // This is what will display underneath our control.  We can
    // format it into a string that makes the most since.
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.2} ms", (self.variance.get() * 1000.)),
            _ => "".to_string(),
        }
    }

    // This shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0  => "Variance",
            _ => "",
        }
        .to_string()
    }
}

// This part is important!  Without it, our plugin won't work.
plugin_main!(Sloth);