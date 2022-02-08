# How to configure 3rd party prompts

## nerdfonts

nerdfonts are not required but they make the presentation much better.

[site](https://www.nerdfonts.com)

[repo](https://github.com/ryanoasis/nerd-fonts)


## oh-my-posh 

[site](ttps://ohmyposh.dev/)

[repo](https://github.com/JanDeDobbeleer/oh-my-posh)


If you like [oh-my-posh](https://ohmyposh.dev/), you can use oh-my-posh with engine-q with few steps. It's works great with engine-q.  There is how to setup oh-my-posh with engine-q:

1. Install Oh My Posh and download oh-my-posh's themes following [guide](https://ohmyposh.dev/docs/linux#installation)
2. Download and Install a [nerd font](https://github.com/ryanoasis/nerd-fonts)
3. Set the PROMPT_COMMAND in ~/.config/nushell/config.nu, change `M365Princess.omp.json` to whatever you like [Themes demo](https://ohmyposh.dev/docs/themes) 
```
let-env PROMPT_COMMAND = { oh-my-posh --config ~/.poshthemes/M365Princess.omp.json }
```
4. Restart engine-q.

## Starship

[site](https://starship.rs/)

[repo](https://github.com/starship/starship)

## Purs

[repo](https://github.com/xcambar/purs)
