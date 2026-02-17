use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Points},
    },
};

use core::f64;
use rand::prelude::*;
use rand_distr::{Distribution, Normal};
use std::time::Duration;

// the screen is 10 meters tall
const WORLD_HEIGHT_IN_METERS: f64 = 1.0;

// earth gravity
const GRAVITY_METERS: f64 = 9.81;

const MARKER_TYPE: Marker = Marker::Braille;
const MARKER_HEIGHT: usize = 4;

#[derive(PartialEq)]
pub enum Message {
    Resize(usize, usize),
    Tick,
    Quit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Running,
    Done,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Material {
    #[default]
    None,
    Sand,
    Water,
    Fire,
    Wood,
    Wall,
}

impl Material {
    // m/sec
    pub fn terminal_velocity(&self) -> f64 {
        match self {
            Material::Sand => 7.0,
            Material::Water => 9.0,
            Material::Wood => 4.0,
            Material::Fire => 3.0,
            Material::None => 0.0,
            Material::Wall => 0.0,
        }
    }

    pub fn density(&self) -> u8 {
        match self {
            Material::None => 0,
            Material::Fire => 1,
            Material::Wood => 8,
            Material::Water => 10,
            Material::Sand => 20,
            Material::Wall => 255,
        }
    }

    // surface friction, more is more
    pub fn friction(&self) -> f64 {
        match self {
            Material::None => 0.0,
            Material::Fire => 0.05,
            Material::Wood => 0.1,
            Material::Sand => 0.5,
            Material::Water => 0.6,
            Material::Wall => 0.0,
        }
    }

    // 0 is sticky, 1 is bouncy
    pub fn bounce(&self) -> f64 {
        match self {
            Material::None => 0.0,
            Material::Water => 0.05,
            Material::Fire => 0.05,
            Material::Sand => 0.05,
            Material::Wood => 0.2,
            Material::Wall => 0.5,
        }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, Material::Sand | Material::Wood | Material::Wall)
    }

    pub fn is_fluid(&self) -> bool {
        matches!(self, Material::Water)
    }

    pub fn is_gas(&self) -> bool {
        matches!(self, Material::Fire)
    }

    pub fn is_combustable(&self) -> bool {
        matches!(self, Material::Wood)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Velocity {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveBuffer {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Particle {
    material: Material,
    velocity: Velocity,
    move_buffer: MoveBuffer,
}

impl Particle {
    pub fn is_empty(&self) -> bool {
        self.material == Material::None
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Anchor {
    /// Fixed pixel coordinates (e.g., x=10, y=10). Good for top-left logos.
    Fixed(usize, usize),

    /// Percentage of screen size (0.0 to 1.0).
    /// (0.5, 0.0) = Top Center. (1.0, 1.0) = Bottom Right.
    Proportional(f64, f64),

    /// Fixed distance from the Right edge.
    /// (10, 5) means "10px from Right, 5px from Top".
    TopRight(usize, usize),

    /// Center X, but Fixed Y.
    /// Good for a main spawner that stays top-center regardless of width.
    CenterX(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spawner {
    x: usize,
    y: usize,
    anchor: Anchor,
    material: Material,
    radius: f64,
    rate: f64,
    accumulator: f64,
    strategy: SpawnStrategy,
    velocity_strategy_x: VelocityStrategy,
    velocity_strategy_y: VelocityStrategy,
}

impl Default for Spawner {
    fn default() -> Self {
        Spawner {
            x: 0,
            y: 0,
            anchor: Anchor::Fixed(0, 0),
            material: Material::Sand,
            radius: 50.0,
            rate: 10.0,
            accumulator: 0.0,
            strategy: SpawnStrategy::Uniform,
            velocity_strategy_x: VelocityStrategy::Fixed(1.0),
            velocity_strategy_y: VelocityStrategy::Fixed(1.0),
        }
    }
}

impl Spawner {
    fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    fn with_material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    fn with_strategy(mut self, strategy: SpawnStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    fn with_velocity_strategy(
        mut self,
        velocity_strategy_x: VelocityStrategy,
        velocity_strategy_y: VelocityStrategy,
    ) -> Self {
        self.velocity_strategy_x = velocity_strategy_x;
        self.velocity_strategy_y = velocity_strategy_y;
        self
    }

    fn with_rate(mut self, rate: f64) -> Self {
        self.rate = rate;
        self
    }

    fn with_radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    fn generate_velocity(&self, strategy: VelocityStrategy) -> f64 {
        match strategy {
            VelocityStrategy::Uniform(lower, upper) => {
                let mut rng = rand::rng();
                rng.random_range(lower..=upper)
            }
            VelocityStrategy::Gaussian(mean, std) => {
                let mut rng = rand::rng();
                let normal = Normal::new(mean, std).unwrap();
                normal.sample(&mut rng)
            }
            VelocityStrategy::Fixed(val) => val,
            VelocityStrategy::None => 0.0,
        }
    }

    fn spawn(&mut self, dt: f64) -> Vec<(usize, usize, Particle)> {
        self.accumulator += dt * self.rate;
        let count = self.accumulator as usize;
        self.accumulator -= count as f64;
        let mut rng = rand::rng();
        let mut particles = vec![];

        for _ in 0..count {
            let (spawn_x, spawn_y) = match self.strategy {
                SpawnStrategy::Point => (self.x as f64, self.y as f64),
                SpawnStrategy::Uniform => {
                    let offset_x = rng.random_range(-self.radius..=self.radius);
                    let rx = self.x as f64 + offset_x;
                    (rx, self.y as f64)
                }
                SpawnStrategy::Gaussian => {
                    let normal = Normal::new(self.x as f64, self.radius / 2.0).unwrap();
                    let rx = normal.sample(&mut rng);
                    (rx, self.y as f64)
                }
            };

            // clamp
            let ix = spawn_x.round() as usize;
            let iy = spawn_y.round() as usize;

            let vel_x = self.generate_velocity(self.velocity_strategy_x);
            let vel_y = self.generate_velocity(self.velocity_strategy_y);
            let particle = Particle {
                material: self.material,
                velocity: Velocity { x: vel_x, y: vel_y },
                move_buffer: MoveBuffer::default(),
            };
            particles.push((ix, iy, particle));
        }

        particles
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpawnStrategy {
    #[default]
    Point,
    Uniform,
    Gaussian,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum VelocityStrategy {
    #[default]
    None,
    Fixed(f64),
    Uniform(f64, f64),
    Gaussian(f64, f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub state: State,
    gravity: f64,
    width: usize,
    height: usize,
    particles: Vec<Particle>,
    spawners: Vec<Spawner>,
}

pub fn default_spawners() -> Vec<Spawner> {
    vec![
        Spawner::default()
            .with_anchor(Anchor::CenterX(0))
            .with_material(Material::Sand)
            .with_radius(75.0)
            .with_rate(10.0)
            .with_strategy(SpawnStrategy::Uniform)
            .with_velocity_strategy(
                VelocityStrategy::Gaussian(0.0, 10.0),
                VelocityStrategy::Uniform(0.0, 10.0),
            ),
        Spawner::default()
            .with_anchor(Anchor::CenterX(0))
            .with_material(Material::Fire)
            .with_radius(75.0)
            .with_rate(10.0)
            .with_velocity_strategy(
                VelocityStrategy::Uniform(-5.0, 5.0),
                VelocityStrategy::Uniform(0.0, 0.0),
            ),
        Spawner::default()
            .with_anchor(Anchor::CenterX(0))
            .with_material(Material::Water)
            .with_velocity_strategy(
                VelocityStrategy::Uniform(-10.0, 10.0),
                VelocityStrategy::Uniform(0.0, 5.0),
            )
            .with_radius(75.0)
            .with_rate(5.0),
        Spawner::default()
            .with_anchor(Anchor::CenterX(0))
            .with_material(Material::Wood)
            .with_radius(75.0)
            .with_rate(10.0)
            .with_velocity_strategy(
                VelocityStrategy::Gaussian(0.0, 10.0),
                VelocityStrategy::Uniform(0.0, 10.0),
            ),
    ]
}

impl Default for Model {
    fn default() -> Self {
        const DEFAULT_SIZE: usize = 150;
        Self::new(DEFAULT_SIZE, DEFAULT_SIZE * MARKER_HEIGHT).with_spawners(default_spawners())
    }
}

impl Model {
    pub fn new(width: usize, height: usize) -> Self {
        Model {
            state: State::default(),
            gravity: GRAVITY_METERS,
            width,
            height,
            particles: vec![Particle::default(); width * height],
            spawners: vec![],
        }
    }

    pub fn with_spawners(mut self, spawners: Vec<Spawner>) -> Self {
        self.spawners = spawners;
        self
    }

    fn ppm(&self) -> f64 {
        self.height as f64 / WORLD_HEIGHT_IN_METERS
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        let new_height = new_height * MARKER_HEIGHT; // 4 accounts for braille height
        if self.width == new_width && self.height == new_height {
            return;
        }

        // new empty grid
        let mut new_particles = vec![Particle::default(); new_width * new_height];

        // centering offsets
        let offset_x = (new_width as isize - self.width as isize) / 2;
        let offset_y = (new_height as isize - self.height as isize) / 2;

        for y in 0..self.height {
            for x in 0..self.width {
                let old_idx = y * self.width + x;

                // skip copying air
                if self.particles[old_idx].material == Material::None {
                    continue;
                }

                // calc new pos
                let new_x = x as isize + offset_x;
                let new_y = y as isize + offset_y;

                // check bounds
                if new_x >= 0
                    && new_x < new_width as isize
                    && new_y >= 0
                    && new_y < new_height as isize
                {
                    let new_idx = (new_y as usize) * new_width + (new_x as usize);
                    let p = self.particles[old_idx];
                    new_particles[new_idx] = p;
                }
            }
        }

        // move the spawners
        for spawner in &mut self.spawners {
            let (calc_x, calc_y) = match spawner.anchor {
                Anchor::Fixed(x, y) => (x, y),
                Anchor::Proportional(u, v) => (
                    (new_width as f64 * u) as usize,
                    (new_height as f64 * v) as usize,
                ),
                Anchor::TopRight(margin_right, y) => (new_width.saturating_sub(margin_right), y),
                Anchor::CenterX(y) => (new_width / 2, y),
            };

            // Safety Clamp (prevent crashing if window gets tiny)
            spawner.x = calc_x.clamp(0, new_width - 1);
            spawner.y = calc_y.clamp(0, new_height - 1);
        }

        // update local state
        self.width = new_width;
        self.height = new_height;
        self.particles = new_particles;
    }

    pub fn update(&mut self, msg: Message, delta: Duration) -> Option<Message> {
        match msg {
            Message::Resize(x, y) => {
                self.resize(x, y);
                None
            }
            Message::Tick => {
                self.update_particles(delta);
                None
            }
            Message::Quit => {
                self.state = State::Done;
                None
            }
        }
    }

    fn get_index(&self, x: isize, y: isize) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            None
        } else {
            let idx = y * self.width as isize + x;
            Some(idx as usize)
        }
    }

    fn set_particle(&mut self, x: usize, y: usize, particle: Particle) {
        let idx = self.get_index(x as isize, y as isize).unwrap();
        self.particles[idx] = particle;
    }

    fn is_empty(&self, x: usize, y: usize) -> bool {
        let idx = self.get_index(x as isize, y as isize).unwrap();
        self.particles[idx].is_empty()
    }

    fn in_bounds(&self, x: isize, y: isize) -> bool {
        x >= 0 && x < self.width as isize && y >= 0 && y < self.height as isize
    }

    fn can_move_to(&self, x1: isize, y1: isize, x2: isize, y2: isize) -> bool {
        if !self.in_bounds(x2, y2) {
            return false;
        }

        if self.is_empty(x2 as usize, y2 as usize) {
            return true;
        }

        let mat1 = match self.get_index(x1, y1) {
            Some(idx) => self.particles[idx].material,
            None => Material::Wall,
        };
        let mat2 = match self.get_index(x2, y2) {
            Some(idx) => self.particles[idx].material,
            None => Material::Wall,
        };

        mat1.density() > mat2.density()
    }
    fn resolve_chemical_reaction(
        &mut self,
        current_x: isize,
        current_y: isize,
        _: isize,
        _: isize,
    ) {
        let current_idx = self.get_index(current_x, current_y).unwrap();
        for neighbor_x in -1..=1 {
            let neighbor_x = neighbor_x + current_x;

            for neighbor_y in -1..=1 {
                let current_mat = self.particles[current_idx].material;
                let neighbor_y = neighbor_y + current_y;

                if neighbor_x == current_x && neighbor_y == current_y {
                    continue;
                }
                if let Some(neighbor_idx) = self.get_index(neighbor_x, neighbor_y) {
                    let neighbor_mat = self.particles[neighbor_idx].material;
                    if current_mat == Material::Fire && neighbor_mat.is_combustable() {
                        self.particles[neighbor_idx].material = Material::Fire;
                    } else if current_mat == Material::Water && neighbor_mat == Material::Fire {
                        self.particles[neighbor_idx] = Particle::default();
                        //self.particles[current_idx] = Particle::default();
                    }
                }
            }
        }
    }

    fn resolve_particle_collision(
        &mut self,
        current_x: isize,
        current_y: isize,
        target_x: isize,
        target_y: isize,
    ) {
        let current_idx = self.get_index(current_x, current_y).unwrap();
        let target_idx = self.get_index(target_x, target_y).unwrap();
        let current_vel = self.particles[current_idx].velocity;
        let current_mat = self.particles[current_idx].material;
        let target_mat = self.particles[target_idx].material;
        let transfer_rate = (current_mat.bounce() + target_mat.bounce()) / 2.0;
        self.particles[target_idx].velocity.x += current_vel.x * transfer_rate;
        self.particles[target_idx].velocity.y += current_vel.y * transfer_rate;
        self.particles[current_idx].velocity.x *= 1.0 - transfer_rate;
        self.particles[current_idx].velocity.y *= 1.0 - transfer_rate;
        self.particles[current_idx].velocity.x *= -target_mat.bounce();
        self.particles[current_idx].velocity.y *= -target_mat.bounce();
        self.particles[current_idx].velocity.x *= 1.0 - target_mat.friction();
        self.particles[current_idx].velocity.y *= 1.0 - target_mat.friction();

        if self.particles[current_idx].velocity.x.abs() < 0.1 {
            self.particles[current_idx].velocity.x = 0.0;
        }
        if self.particles[current_idx].velocity.y.abs() < 0.1 {
            self.particles[current_idx].velocity.y = 0.0;
        }
    }

    fn get_material(&self, x: isize, y: isize) -> Material {
        match self.get_index(x, y) {
            Some(idx) => self.particles[idx].material,
            None => Material::Wall,
        }
    }

    fn resolve_wall_collision(
        &mut self,
        current_x: isize,
        current_y: isize,
        target_x: isize,
        target_y: isize,
    ) {
        let current_idx = self.get_index(current_x, current_y).unwrap();
        let hit_x = !self.can_move_to(current_x, current_y, target_x, current_y);
        let current_material = self.particles[current_idx].material;

        if hit_x {
            let target_material_x = self.get_material(target_x, current_y);
            let avg_bounce = (target_material_x.bounce() + current_material.bounce()) / 2.0;
            self.particles[current_idx].velocity.x *= -avg_bounce;
            self.particles[current_idx].velocity.y *= (1.0 - target_material_x.friction()).max(0.0);
        }

        let hit_y = !self.can_move_to(current_x, current_y, current_x, target_y);
        if hit_y {
            let target_material_y = self.get_material(current_x, target_y);
            let avg_bounce = (target_material_y.bounce() + current_material.bounce()) / 2.0;
            self.particles[current_idx].velocity.y *= -avg_bounce;
            self.particles[current_idx].velocity.x *= (1.0 - target_material_y.friction()).max(0.0);
        }

        if !hit_x && !hit_y {
            self.particles[current_idx].velocity = Velocity { x: 0.0, y: 0.0 };
        }
    }

    fn swap_particles(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let idx1 = self.get_index(x1 as isize, y1 as isize).unwrap();
        let idx2 = self.get_index(x2 as isize, y2 as isize).unwrap();
        self.particles.swap(idx1, idx2);
    }

    fn spawn(&mut self, dt: f64) {
        let mut pending = Vec::new();

        for spawner in &mut self.spawners {
            pending.extend(spawner.spawn(dt));
        }

        for (x, y, spawned) in pending {
            if self.in_bounds(x as isize, y as isize) && self.is_empty(x, y) {
                self.set_particle(x, y, spawned);
            }
        }
    }

    fn update_particle(&mut self, dt: f64, x: usize, y: usize) {
        let ppm = self.ppm();
        let idx = self.get_index(x as isize, y as isize).unwrap();

        let term_vel_meters = self.particles[idx].material.terminal_velocity();
        let term_vel_pixels = term_vel_meters * ppm;
        let drag_coeff = (self.gravity * ppm) / (term_vel_pixels * term_vel_pixels);

        let vx = self.particles[idx].velocity.x;
        let vy = self.particles[idx].velocity.y;

        let drag_x = -drag_coeff * vx * vx.abs();
        let drag_y = -drag_coeff * vy * vy.abs();

        let new_vx = vx + (drag_x * dt);
        let new_vy = vy + (self.gravity + drag_y) * dt;

        self.particles[idx].velocity.x = new_vx;
        self.particles[idx].velocity.y = new_vy;
        if self.particles[idx].velocity.x.abs() < 0.1 {
            self.particles[idx].velocity.x = 0.0;
        }
        if self.particles[idx].velocity.y.abs() < 0.1 {
            self.particles[idx].velocity.y = 0.0;
        }

        // accumulate distance
        self.particles[idx].move_buffer.x += new_vx * dt;
        self.particles[idx].move_buffer.y += new_vy * dt;

        // calculate steps to take
        let mut steps_y = self.particles[idx].move_buffer.y.trunc() as isize;
        let mut steps_x = self.particles[idx].move_buffer.x.trunc() as isize;
        let mut current_y = y as isize;
        let mut current_x = x as isize;

        // decrease the buffer
        while steps_y.abs() >= 1 || steps_x.abs() >= 1 {
            let mut next_x = current_x;
            let mut next_y = current_y;

            if steps_x.abs() >= 1 {
                let step_x = steps_x.signum();
                self.particles[idx].move_buffer.x -= step_x as f64;
                next_x += step_x;
                steps_x -= step_x;
            }

            if steps_y.abs() >= 1 {
                let step_y = steps_y.signum();
                self.particles[idx].move_buffer.y -= step_y as f64;
                next_y += step_y;
                steps_y -= step_y;
            }

            if !self.in_bounds(next_x, next_y) {
                // hit a wall
                self.resolve_wall_collision(current_x, current_y, next_x, next_y);
                steps_x = 0;
                steps_y = 0;
            } else if self.is_empty(next_x as usize, next_y as usize) {
                // hit empty space
                self.swap_particles(
                    current_x as usize,
                    current_y as usize,
                    next_x as usize,
                    next_y as usize,
                );
                current_x = next_x;
                current_y = next_y;
            } else {
                // hit another particle
                let current_idx = self.get_index(current_x, current_y).unwrap();
                let target_idx = self.get_index(next_x, next_y).unwrap();
                let current_density = self.particles[current_idx].material.density();
                let other_density = self.particles[target_idx].material.density();

                if current_density > other_density {
                    // displace something / sink
                    let friction = 1.0 - self.particles[target_idx].material.friction();
                    self.swap_particles(
                        current_x as usize,
                        current_y as usize,
                        next_x as usize,
                        next_y as usize,
                    );
                    self.particles[target_idx].velocity.x *= friction;
                    self.particles[target_idx].velocity.y *= friction;
                    current_x = next_x;
                    current_y = next_y;
                } else {
                    // impact
                    self.resolve_particle_collision(current_x, current_y, next_x, next_y);
                    self.resolve_chemical_reaction(current_x, current_y, next_x, next_y);

                    let velocity = self.particles[current_idx].velocity;
                    let material = self.particles[current_idx].material;
                    let open_below =
                        self.can_move_to(current_x, current_y, current_x, current_y + 1);
                    let open_diag_left =
                        self.can_move_to(current_x, current_y, current_x - 1, current_y + 1);
                    let open_diag_right =
                        self.can_move_to(current_x, current_y, current_x + 1, current_y + 1);
                    let open_left =
                        self.can_move_to(current_x, current_y, current_x - 1, current_y);
                    let open_right =
                        self.can_move_to(current_x, current_y, current_x + 1, current_y);

                    if velocity.y == 0.0 && !open_below {
                        let targ = if open_diag_left {
                            Some((current_x as usize - 1, current_y as usize + 1))
                        } else if open_diag_right {
                            Some((current_x as usize + 1, current_y as usize + 1))
                        } else if material.is_fluid() && open_left {
                            Some((current_x as usize - 1, current_y as usize))
                        } else if material.is_fluid() && open_right {
                            Some((current_x as usize + 1, current_y as usize))
                        } else {
                            None
                        };
                        if let Some((targ_x, targ_y)) = targ {
                            let friction = self.get_material(current_x, current_y + 1).friction();
                            self.swap_particles(
                                current_x as usize,
                                current_y as usize,
                                targ_x,
                                targ_y,
                            );

                            self.particles[target_idx].velocity.x *= friction;
                            self.particles[target_idx].velocity.y *= friction;
                        }
                    } else {
                        steps_x = 0;
                        steps_y = 0;
                    }
                }
            }
        }
    }

    fn update_particles(&mut self, delta: Duration) {
        let dt = delta.as_secs_f64();
        self.spawn(dt);

        // update particles from the bottom row up
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                if !self.is_empty(x, y) {
                    self.update_particle(dt, x, y);
                };
            }
        }
    }

    fn get_material_coords(&self, material: Material) -> Vec<(f64, f64)> {
        self.particles
            .iter()
            .enumerate()
            .filter_map(|(i, particle)| {
                if particle.material == material {
                    let x = (i % self.width) as f64;
                    let y = (i / self.width) as f64;
                    // ratatui canvas needs the y inverted
                    Some((x, self.height as f64 - 1.0 - y))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Widget for &Model {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let canvas = Canvas::default()
            .marker(MARKER_TYPE)
            .x_bounds([0.0, self.width as f64])
            .y_bounds([0.0, self.height as f64])
            .paint(move |ctx| {
                ctx.draw(&Points {
                    coords: &self.get_material_coords(Material::Sand),
                    color: Color::White,
                });

                ctx.draw(&Points {
                    coords: &self.get_material_coords(Material::Water),
                    color: Color::Blue,
                });

                ctx.draw(&Points {
                    coords: &self.get_material_coords(Material::Fire),
                    color: Color::Red,
                });

                ctx.draw(&Points {
                    coords: &self.get_material_coords(Material::Wood),
                    color: Color::Yellow,
                });
            });

        canvas.render(area, buf);
    }
}
