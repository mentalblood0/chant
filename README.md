# 🔮 chant

[![tests](https://github.com/mentalblood0/chant/actions/workflows/build.yml/badge.svg)](https://github.com/mentalblood0/chant/actions/workflows/build.yml)

Telegram collaboration interface to [wool](https://github.com/mentalblood0/wool) theses storage

## Workflow

- the offerer or the cantor sends message with commands batch to the bot
- the bot forwards this message to all cantors
- when any cantor likes message forwarded to her, bot executes corresponding commands batch, deletes all corresponding forwarded messages and likes source message
