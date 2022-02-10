# How to configure 3rd party prompts

## nerdfonts

nerdfonts are not required but they make the presentation much better.

[site](https://www.nerdfonts.com)

[repo](https://github.com/ryanoasis/nerd-fonts)

## oh-my-posh

[site](ttps://ohmyposh.dev/)

[repo](https://github.com/JanDeDobbeleer/oh-my-posh)

If you like [oh-my-posh](https://ohmyposh.dev/), you can use oh-my-posh with nushell with few steps. It's works great with nushell. There is how to setup oh-my-posh with nushell:

1. Install Oh My Posh and download oh-my-posh's themes following [guide](https://ohmyposh.dev/docs/linux#installation).
2. Download and Install a [nerd font](https://github.com/ryanoasis/nerd-fonts).
3. Set the PROMPT_COMMAND in ~/.config/nushell/config.nu, change `M365Princess.omp.json` to whatever you like [Themes demo](https://ohmyposh.dev/docs/themes).

```
let_env PROMPT_COMMAND = { oh-my-posh --config ~/.poshthemes/M365Princess.omp.json }
```

## Starship

[site](https://starship.rs/)

[repo](https://github.com/starship/starship)

1. Follow the links above and install starship.
2. Install nerdfonts depending on your preferences.
3. If you want the default ticking clock with date & time on the right prompt execut this command `hide PROMPT_COMMAND_RIGHT`
4. If you don't want the default indicator, you can run this command `let_env PROMPT_INDICATOR = " "`
5. Set starship as your left prompt with this command `let_env PROMPT_COMMAND = { starship prompt --cmd-duration $env.CMD_DURATION_MS | str trim }`. Note that you may not have to use `str trim` in the nushell prompt if you disable starship's default newline setting with this entry in the starship.toml file `add_newline = false`. There have been reports that this might not play nice with nushell prompts. We're still testing.
6. Since nushell supports a right prompt you can also play around with starship's ability to set a right prompt. Setting the right prompt in nushell is identical to setting the left prompt however you use `PROMPT_COMMAND_RIGHT`.

## Purs

[repo](https://github.com/xcambar/purs)
