use rand::Rng;

use crate::prelude::Point;

use super::Vertex;
use super::Meshable;

pub(super) struct Planetoid {
  pos: Point,
  radius: f32,
  color: [f32; 3],
  vertices: Vec<Vertex>
}

impl Planetoid {
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

impl Meshable for Planetoid {
    fn vertices(&self) -> Vec<Vertex> {
        self.vertices.clone()
    }

    fn indices(&self) -> Vec<u16> {
        Self::INDICES.to_vec()
    }

    fn recalculate_vertices(&mut self, pos: Point) {
        self.pos = pos;
        
        self.vertices.clear();

        // 1st add the center point
        self.vertices.push(
            Vertex {
                position: [
                    self.pos.x(),
                    self.pos.y(),
                    0f32
                ],
                color: self.color
            }
        );

        // AND the 1st point on the circumference of the circle
        self.vertices.push(
            Vertex {
                position: [
                    self.radius + self.pos.x(),
                    self.pos.y(),
                    0f32 
                ],
                color: self.color
            }
        );

        // Add in each slice, one by one
        for i in (19625..628000).step_by(19625) {
            let i = i as f32 * 0.00001f32;

            self.vertices.push(
                Vertex {
                    position: [
                        i.cos() * self.radius + self.pos.x(),
                        i.sin() * self.radius + self.pos.y(),
                        0f32
                    ],
                    color: self.color
                }
            );
        }
    }
}

impl Planetoid {
  pub(super) fn new(pos: Point, radius: f32) -> Self {
      let mut planetoid = Self {
          pos,
          radius,
          color: [
              rand::thread_rng().gen::<f32>(),
              rand::thread_rng().gen::<f32>(),
              rand::thread_rng().gen::<f32>(),
          ],
          vertices: Vec::new()
      };

      planetoid.recalculate_vertices(pos);

      planetoid
  }
}
