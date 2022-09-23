use rand::Rng;
use rand_seeder::SipHasher;

use super::Vertex;

pub(super) trait Meshable {
    fn vertices(&self) -> Vec<Vertex>;
    fn indices(&self) -> Vec<u16>;
}

impl crate::simulation::planet::Planet {
    const INDICES: &'static [u16] = &[
      1, 2, 0, 
      2, 3, 0, 
      3, 4, 0, 
      4, 5, 0, 
      5, 6, 0, 
      6, 7, 0, 
      7, 8, 0, 
      8, 9, 0, 
      9, 10, 0, 
      10, 11, 0, 
      11, 12, 0, 
      12, 13, 0, 
      13, 14, 0, 
      14, 15, 0, 
      15, 16, 0, 
      16, 17, 0, 
      17, 18, 0, 
      18, 19, 0, 
      19, 20, 0, 
      20, 21, 0, 
      21, 22, 0, 
      22, 23, 0, 
      23, 24, 0, 
      24, 25, 0, 
      25, 26, 0, 
      26, 27, 0, 
      27, 28, 0, 
      28, 29, 0, 
      29, 30, 0, 
      30, 31, 0, 
      31, 32, 0, 
      32, 1, 0
  ];
}

impl Meshable for crate::simulation::planet::Planet {
    fn vertices(&self) -> Vec<Vertex> {
        let mut vertices = Vec::new();

        let mut prng = SipHasher::from(self).into_rng();

        let color = [
            prng.gen_range(0f32..1f32),
            prng.gen_range(0f32..1f32),
            prng.gen_range(0f32..1f32)
        ];


        // 1st add the center point
        vertices.push(
            Vertex {
                position: [
                    self.pos().x,
                    self.pos().y,
                    0f32
                ],
                color
            }
        );

        // AND the 1st point on the circumference of the circle
        vertices.push(
            Vertex {
                position: [
                    self.radius() + self.pos().x,
                    self.pos().y,
                    0f32 
                ],
                color
            }
        );

        // Add in each slice, one by one
        for i in (19625..628000).step_by(19625) {
            let i = i as f32 * 0.00001f32;

            vertices.push(
                Vertex {
                    position: [
                        i.cos() * self.radius() + self.pos().x,
                        i.sin() * self.radius() + self.pos().y,
                        0f32
                    ],
                    color
                }
            );
        }

        vertices
    }

    fn indices(&self) -> Vec<u16> {
        Self::INDICES.to_vec()
    }
}

impl Meshable for crate::simulation::ship::Ship {
    fn vertices(&self) -> Vec<Vertex> {
        use crate::simulation::ship::ShipBehavior;
        let color = match self.behavior() {
            ShipBehavior::Miner => [1f32, 0.4f32, 0f32],
            ShipBehavior::Trader => [0f32, 0.6f32, 1f32],
            ShipBehavior::Pirate => [1f32, 0f32, 0f32]
        };

        let min_angle = self.angle() - 0.2617994;
        let max_angle = self.angle() + 0.2617994;

        let size = crate::simulation::planet::Planet::SUN_RADIUS * 0.5;

        vec![
            Vertex { position: [ self.pos().x, self.pos().y, 0f32 ], color },
            Vertex { position: [ size * min_angle.sin() + self.pos().x, size * min_angle.cos() + self.pos().y, 0f32 ], color: [ 0f32, 0f32, 0f32 ] },
            Vertex { position: [ size * max_angle.sin() + self.pos().x, size * max_angle.cos() + self.pos().y, 0f32 ], color: [ 0f32, 0f32, 0f32 ] },
        ]
    }

    fn indices(&self) -> Vec<u16> {
        vec![0, 1, 2]
    }
}