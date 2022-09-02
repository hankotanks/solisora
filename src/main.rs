mod facade;
mod simulation;

fn main() {
    let simulation = simulation::Simulation::default();
    
    pollster::block_on(
        facade::run(simulation)
    );
}