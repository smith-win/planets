
// Sun -- 1.98892 x 10 ^ 30 kg;
// Earth
//   149,597,870 -- km from sun!!
//   107,226 km/h -- speed (so it starting at "zero") -- this is the distance
//   5.972 × 10^24 kg
//   moon 7.34767309 × 10^22 kg

use std::sync::{Arc, Mutex};
use std::thread;

use femtovg::{
    renderer::OpenGl,
    Canvas,
    Color,
    Paint,
    Path,
};


use glutin::event::{
    Event,
};
use glutin::event_loop::{
    ControlFlow,
    EventLoop,
};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;

use euclid::Vector2D;



/// Tagging type for position
/// Tagging types used because of the euclids crate that puts an extra type ("unit")
/// to prevent you mixing e.g Force and something else inappropriately
struct Position {}
type PositionVec = Vector2D<f64, Position>;

/// Tagging type for position
struct Velocity {}
type VelocityVec = Vector2D<f64, Velocity>;

/// Tagging type for a force
struct Force {}
type ForceVec = Vector2D<f64, Force>;

// A Body has mass, (immutable)
// a position, and a velocity
struct Body {

    display_colour: Color,
    display_radius: f32,
    mass: f64,

    position: PositionVec,
    velocity: VelocityVec

}


// kilometers to edge of system
const SYSTEM_EXTENT_KM : f32 = 250_000_000.0 * 1000.0;

/// Gravitational constant
const G : f64 = 6.673e-11;

fn main() {
    
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_inner_size(glutin::dpi::PhysicalSize::new(1000, 600))
        .with_title("Silly N-Body Simulator");

    let windowed_context = ContextBuilder::new().with_vsync(false).build_windowed(wb, &el).unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let renderer = OpenGl::new(|s| windowed_context.get_proc_address(s) as *const _).expect("Cannot create renderer");
    let mut canvas = Canvas::new(renderer).expect("Cannot create canvas");    


    // setup the planets
    let sun = Body {
        display_colour: Color::rgb(200, 200, 0),
        display_radius: 10.0,
        mass: 1.98892e30,
        position: PositionVec::new(0.0, 0.0), // Sun is centre of universe!
        velocity: VelocityVec::new(0.0, 0.0)
    };


    let mercury = Body {
        display_colour: Color::rgb(200, 200, 200),
        display_radius: 2.0,
        mass: 3.285e25,
        position: PositionVec::new(-67_454_000.0 * 1000.0, 0.0 /* km  to m*/ ),
        velocity: VelocityVec::new(0.0, -47.0 * 1000.0 )  
    };



    let earth = Body {
        display_colour: Color::rgb(50, 50, 200),
        display_radius: 5.0,
        mass: 5.972e24,
        position: PositionVec::new(-149_597_870.0 * 1000.0, 0.0 /* km  to m*/ ),
        velocity: VelocityVec::new(0.0, -107226.0 * 1000.0 / (60.0 * 60.0))  // km per hour = meters per secod
    };

    // speed = 35 km/s
    let venus = Body {
        display_colour: Color::rgb(100, 100, 100),
        display_radius: 4.0,
        mass: 4.867e24,
        position: PositionVec::new(108_570_000.0 * 1000.0, 0.0),
        velocity: VelocityVec::new(0.0, 35.26 * 1000.0)
    };

    // speed = 35 km/s
    let mars = Body {
        display_colour: Color::rgb(200, 50, 50),
        display_radius: 4.0,
        mass: 6.39e23,
        position: PositionVec::new(0.0, -249_250_000.0 * 1000.0),
        velocity: VelocityVec::new(24.07 * 1000.0, 0.0)
    };
    
    // let jupiter = Body {
    //     display_colour: Color::rgb(150, 150, 150),
    //     display_radius: 20.0,
    //     position: PositionVec::new(550.0, 100.0),
    //     velocity: VelocityVec::new(0.0, 0.0)
    // };

    // The bodies will be used by the updating thread, and also the 
    // drawing thread
    let bodies = vec![sun, mercury, venus, earth, mars];

    let bodies = Arc::new(Mutex::new(bodies));

    let evt_loop_bodies = bodies.clone();


    // thread for updating positions, using glutin event loop proxy
    
    let el_proxy = el.create_proxy();
    let _ = thread::spawn( move|| -> () {

        // prepare a vec for the forces on each body in the system
        // doing in this style means the mutexguard here is dropped
        let mut force_vecs = {
            let data = bodies.lock().unwrap();
            let mut v = Vec::with_capacity(data.len());
            for _b in &*data {
                v.push( ForceVec::new(0.0, 0.0) );
            }
            v
        };

        // pause the animation
        loop {
            // pause for a while
            thread::sleep(std::time::Duration::from_millis(50));

            let mut data = bodies.lock().unwrap(); // I think this blocks!!
            assert!(force_vecs.len() == data.len()); // prevent range checking

            let n = data.len();
            for i in 0..n {
                // Reset force acting on the body
                let net_force = &mut force_vecs[i];
                net_force.x = 0.0;
                net_force.y = 0.0;

                for j in 0..n {

                    if i != j {
                        // work out distance between two bodies 
                        let eps:f64 = 3e4;
                        // multiply by 1000 as distances I entered were in Km .. needs to be metres??
                        let dx = data[j].position.x - data[i].position.x;
                        let dy = data[j].position.y - data[i].position.y;

                        let d = (dx*dx+ dy*dy).sqrt();  

                        //println!("i={}, dx={}, dy={}, d={}", i, dx, dy, d);

                        // TODO -- softener - see what this is
                        let f = (G * data[i].mass * data[j].mass) as f64 / (d*d + eps*eps);
                        net_force.x += f * dx / d;
                        net_force.y += f * dy / d;
                        //println!("i={}, f={}, netforce {:?}", i, f, net_force);
                    }
                }
            }

            // update all velocities, then positions based on forces
            let dt:f64 = 1.0 * 60.0 * 60.0 * 12.0; // half earth day as our time slice

            // println!("data.len {}", data.len());
            for i in 0..data.len() {
                data[i].velocity.x += dt * force_vecs[i].x / data[i].mass;
                data[i].velocity.y += dt * force_vecs[i].y / data[i].mass;
                // println!("i={}, velocity {:?}", i, data[i].velocity);
                
                data[i].position.x += dt * data[i].velocity.x;
                data[i].position.y += dt * data[i].velocity.y;

                // println!("i={}  x={}", i, data[i].position.x);
            };
            // println!();

            // update the display
            if el_proxy.send_event(()).is_err() {
                // exit if event loop gone
                return;
            }
        }

    });


    el.run( move | event, _, control_flow|  {

        // wait for another event -- see if this prevents thrashing
        // yes it does - much nicer

        // see how "breakout" does timing - updates this .. 
        *control_flow = ControlFlow::Wait;


        // update_positions(&mut bodies);

        match event {

            Event::RedrawRequested(_) | Event::UserEvent(_)=> {

                // should delegate to a redraw function here
                let dpi_factor = windowed_context.window().scale_factor();
                let size = windowed_context.window().inner_size();
                
                canvas.set_size(size.width as u32, size.height as u32, dpi_factor as f32);
                canvas.clear_rect(0, 0, size.width as u32, size.height as u32, Color::rgbf(0.1, 0.1, 0.2));

                let cx = size.width / 2 ;
                let cy = size.height / 2 ;

                let scale = cy as f32 / SYSTEM_EXTENT_KM;

                // Remember to unlock the bodies
                let data = evt_loop_bodies.lock().unwrap();

                for b in &*data {
                    let mut path = Path::new();
                    let _circle = path.circle(cx as f32 + scale * b.position.x as f32, cy as f32 + scale * b.position.y as f32, b.display_radius);
                    let p = Paint::color(b.display_colour);
                    canvas.fill_path(&mut path, p);

                    // // Label for the planet
                    // let p = Paint::color(Color::rgbf(1.0, 1.0, 1.0));
                    // canvas.stroke_text(cx as f32 + scale * b.position.x as f32, 10.0+(cy as f32 + scale * b.position.y as f32), "Planet", p);

                }

                canvas.flush();
                windowed_context.swap_buffers().unwrap();

            },

            // This is the wai I try to kill the window
            Event::WindowEvent{ window_id: _id, event: e } => {
                match e {
                    glutin::event::WindowEvent::CloseRequested => { *control_flow = ControlFlow::Exit },
                    glutin::event::WindowEvent::Destroyed => { *control_flow = ControlFlow::Exit },
                    _ => {} /* ignore rest */
                }
            },

            _ => {}
        }

    });
}
