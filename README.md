# falling-sand
[![Asciinema Recording](https://asciinema.org/a/788929.png)](https://asciinema.org/a/788929)

## Particles

- **Sand**: High friction, rolls down hills, not flammable
- **Water**: Flows sidways, puts out fire and evaporates
- **Fire**: Spreads to adjacent flammables
- **Wood**: Flammable, less dense than sand or water

## Physics

- World modeled as 10 meters tall
- Earth gravity
- Each particle assigned:
  - Terminal velocity (m/s)
  - Density (1-100, higher is more dense)
  - Bounciness (0.0 [mud] - 1.0 [super bounce ball])
  - Surface friction (0.0 [slick] - 1.0 [slick])
- Denser particles sink, swapping with and displacing less dense ones
- Collision with particle of lesser or equal density results in energy transfer
