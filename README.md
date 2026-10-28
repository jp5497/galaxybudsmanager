# Galaxy Buds 4 Pro Manager (`buds4pro`)

Gerenciador completo, nativo e ultraleve em **Rust** focado no **Samsung Galaxy Buds 4 Pro (`SM-R640`)** para Linux.

## Recursos

- **Interface Gráfica Desktop (GUI)**: Painel moderno em tema escuro (`eframe` / `egui`) compatível nativamente com Wayland e X11.
- **Bandeja do Sistema (System Tray)**: Integração nativa via `StatusNotifierItem` (`ksni`) no KDE Plasma / ambientes Linux com leitura de bateria em tempo real e troca rápida de modos com 1 clique.
- **CLI Rápida & Servidor IPC**: Comunicação instantânea (`< 1 ms`) via Unix Socket (`/tmp/buds4pro-manager.sock`) quando o daemon/GUI está aberto, ou conexão Bluetooth direta sob demanda.
- **Suporte completo ao protocolo SPP do Galaxy Buds 4 Pro (`SM-R640`)**:
  - Bateria individual (Fone Esquerdo, Direito e Estojo) + estado de carregamento e uso no ouvido.
  - Sensores internos de temperatura (°C), revisão de hardware, versão de firmware (`R640...`), SKU e números de série.
  - Controle de Ruído: **ANC (Cancelamento Ativo de Ruído)**, **Som Ambiente**, **Adaptativo** e **Desligado**.
  - Ajuste de nível de Som Ambiente (`0` a `4`) e nível de ANC (`0` a `4`).
  - **Equalizador**: Presets (`Dinâmico`, `Bass Boost`, `Claro`, `Suave`, `Treble Boost`, `Desligado`) + **Equalizador Personalizado de 9 Bandas** (`-10` a `+10 dB`).
  - **Detecção de Conversas (Voice Detect)** com ajuste de tempo (`5s`, `10s`, `15s`).
  - Bloqueio de controles de toque/pinça na haste (geral ou por gesto).
  - **Modo Jogo** (baixa latência), **Conexão Contínua (Seamless Connection)**, **Sidetone**, **Chamadas com Som Extra Claro** e **Encontrar Meus Fones**.

---

## Instalação

### Via Release Binário (Linux x86_64)
Baixe o binário na aba **[Releases](https://github.com/jp5497/galaxybudsmanager/releases)**:

```bash
chmod +x buds4pro-linux-x86_64
mv buds4pro-linux-x86_64 ~/.local/bin/buds4pro
```

### Compilando do Código-Fonte
Requisitos no Arch Linux: `rust`, `bluez`, `dbus`, `wayland` / `libx11`.

```bash
git clone https://github.com/jp5497/galaxybudsmanager.git
cd galaxybudsmanager
cargo build --release
cp target/release/buds4pro ~/.local/bin/
cp assets/buds4pro.desktop ~/.local/share/applications/
```

---

## Comandos Principais

```bash
# Abrir a Interface Gráfica (GUI) + System Tray (comportamento padrão)
buds4pro
# ou
buds4pro gui

# Rodar apenas como daemon em segundo plano + System Tray
buds4pro daemon

# Ver status completo no terminal
buds4pro status

# Ver status em JSON (para Waybar / Polybar / scripts)
buds4pro status --json

# Alternar Controle de Ruído
buds4pro noise anc
buds4pro noise ambient
buds4pro noise adaptive
buds4pro noise off
buds4pro noise cycle

# Ajustar nível de Som Ambiente (0 a 4) ou ANC (0 a 4)
buds4pro ambient-volume 4
buds4pro anc-level 4

# Alterar Equalizador
buds4pro eq dynamic
buds4pro eq bass
buds4pro eq clear
buds4pro eq custom

# Definir Equalizador Personalizado de 9 bandas (-10 a +10 dB)
buds4pro custom-eq 4 3 1 0 -1 0 2 3 4

# Ativar/Desativar Detecção de Conversas (Voice Detect)
buds4pro voice-detect on --timeout 10
buds4pro voice-detect off

# Bloquear/Desbloquear controles de toque na haste
buds4pro touchpad lock
buds4pro touchpad unlock

# Modo Jogo (Baixa Latência)
buds4pro game-mode on
buds4pro game-mode off

# Encontrar Meus Fones (Alerta Sonoro)
buds4pro find start
buds4pro find stop
```
