# 🔮 chant

[![tests](https://github.com/mentalblood0/chant/actions/workflows/build.yml/badge.svg)](https://github.com/mentalblood0/chant/actions/workflows/build.yml)

Telegram collaboration interface to [wool](https://github.com/mentalblood0/wool) theses storage

## Overview

There are two user roles available: Offerer and Cantor

### Offerer

An Offerer can request execution of some storage-altering commands by sending them in file with `.txt` extension (commands syntax described in [wool readme](https://github.com/mentalblood0/wool#commands)). Execution of storage-altering commands is postponed until approval from a user with Cantor role. Offerer can also execute read commands: get full thesis information by it's identifier or alias, search theses by tags

#### In-message commands

`/reference one_reference_to_search_by`

`/tags one or more some tags to search by`

Reference is either thesis identifier or thesis alias

Basic concepts described in [wool readme](https://github.com/mentalblood0/wool#basic-concepts)

**Theses aliases and identifiers in bot replies to `/reference` and `/tags` commands are clickable**

### Cantor

A Cantor is an Offerer which also receives approval requests for storage-altering commands. These requests sent by bot immediately after bot received them from an Offerer. Approvement done by just liking the corresponding message. Disapprovement done by just disliking the corresponding message

Cantor also have some specific in-message commands:

`/add_offerers one or more telegram user identifiers`

`/promote_to_cantor one_offerer_telegram_user_identifier`

Telegram user identifier is an integer number, i.e. `810722109`
