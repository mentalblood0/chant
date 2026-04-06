# 🔮 chant

[![tests](https://github.com/mentalblood0/chant/actions/workflows/build.yml/badge.svg)](https://github.com/mentalblood0/chant/actions/workflows/build.yml)

Telegram collaboration interface to [wool](https://github.com/mentalblood0/wool) theses storage

## Overview

There are two user roles available: Offerer and Cantor

An Offerer can request execution of some storage-altering commands by sending them in file with `.txt` extension. Execution of storage-altering commands is postponed until approval from a user with Cantor role. Offerer can also execute read commands: get full thesis information by it's identifier or alias, search theses by tags

A Cantor is an Offerer which also receives approval requests for storage-altering commands. These requests sent by bot immediately after bot received them from an Offerer. Approvement done by just liking the corresponding message. Disapprovement done by just disliking the corresponding message

For storage-altering commands syntax see [wool readme](https://github.com/mentalblood0/wool#commands)
