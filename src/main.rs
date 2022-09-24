mod sim;
mod ui;

fn main() {
    let sim = sim::Sim::default();
    
    pollster::block_on(
        ui::run(sim)
    );
}
