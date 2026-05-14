# falling-sand

[![Asciinema Recording](https://asciinema.org/a/788929.png)](https://asciinema.org/a/788929)

A little falling-sand simulation that runs in the terminal — sand piles up, water
flows, fire spreads, wood burns. Built with [Ratatui](https://ratatui.rs/) as a
quick experiment in what you can do in a terminal without much effort.

## Running

```
cargo run
```

Press `q` to quit.

## Particles

- **Sand**: High friction, rolls down hills, not flammable
- **Water**: Flows sideways, puts out fire and evaporates
- **Fire**: Spreads to adjacent flammables
- **Wood**: Flammable, less dense than sand or water

## Physics

- World modeled as 10 meters tall
- Earth gravity
- Each particle assigned:
  - Terminal velocity (m/s)
  - Density (1-100, higher is more dense)
  - Bounciness (0.0 [mud] - 1.0 [super bounce ball])
  - Surface friction (0.0 [slick] - 1.0 [grippy])
- Denser particles sink, swapping with and displacing less dense ones
- Collision with particle of lesser or equal density results in energy transfer

## License

MIT
