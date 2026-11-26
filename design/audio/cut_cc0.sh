#!/bin/bash
# Cuts the CC0 recordings into the game's sounds (assets/audio/cc0/, credits in
# assets/audio/CREDITS.md). Run from anywhere: `design/audio/cut_cc0.sh`.
# Needs ffmpeg and the raw downloads in design/audio/raw/ (git-ignored: the Freesound files keep
# their `<id>__<author>__<title>` names, the Kenney packs are unzipped as rpg-audio/, impact-sounds/).
# Check the result with `design/audio/cc0_report.sh`.
set -euo pipefail
cd "$(dirname "$0")/../.."
FF=${FFMPEG:-ffmpeg}
RAW=${RAW:-design/audio/raw}
FS=$RAW/freesound
K=$RAW/kenney/rpg-audio/Audio
KI=$RAW/kenney/impact-sounds/Audio
OUT=assets/audio/cc0
TMP=$(mktemp -d -t shiftella-cc0)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT" "$TMP"
rm -f "$OUT"/*.wav "$OUT"/*.ogg
TARGET=-24      # LUFS: the median of the generated SFX (cash register -22, coffee -24, alert -25)
CEIL=-1.5       # dBTP ceiling

ff() { $FF -hide_banner -v error -y "$@"; }

# cut ID SRC START DUR FADEIN FADEOUT [FILTERS] -> $TMP/ID.pre.wav (mono float, 44.1 kHz)
cut() {
  local id=$1 src=$2 ss=$3 d=$4 fi=$5 fo=$6 extra=${7:-anull}
  local st; st=$(awk "BEGIN{printf \"%.4f\", $d - $fo}")
  ff -ss "$ss" -t "$d" -i "$src" -af "aformat=sample_fmts=flt,$extra,afade=t=in:d=$fi:curve=qsin,afade=t=out:st=$st:d=$fo:curve=qsin" \
     -ac 1 -ar 44100 -c:a pcm_f32le "$TMP/$id.pre.wav"
}

# measure FILE -> "I TP"
measure() {
  $FF -hide_banner -nostats -i "$1" -af "apad=pad_dur=0.5,ebur128=peak=true" -f null - 2>&1 \
    | awk '/Summary/{s=1} s&&/I:/{i=$2} s&&/Peak:/{p=$2} END{print i, p}'
}

# finish ID wav|ogg [channels]: normalise to TARGET with a peak ceiling, write the final file
finish() {
  local id=$1 fmt=$2 pre="$TMP/$1.pre.wav" mid="$TMP/$1.mid.wav"
  # the encoder overshoots a little: loops keep more headroom
  local ceil=$CEIL; [ "$fmt" = ogg ] && ceil=-3
  local limit; limit=$(awk "BEGIN{printf \"%.4f\", 10^(($ceil)/20)}")
  read -r i p < <(measure "$pre")
  local g; g=$(awk "BEGIN{printf \"%.2f\", $TARGET - ($i)}")
  # a peaky sound (a click, a knock) is limited gently instead of pushing its peak past the
  # ceiling; the limiter takes a little loudness away, so a few passes settle the gain
  local pass
  for pass in 1 2 3 4 5; do
    ff -i "$pre" -af "volume=${g}dB,alimiter=limit=$limit:attack=1.5:release=40:level=false:latency=1" -c:a pcm_f32le "$mid"
    read -r i p < <(measure "$mid")
    awk "BEGIN{d=$TARGET-($i); exit !(d<0.4 && d>-0.4)}" && break
    g=$(awk "BEGIN{printf \"%.2f\", $g + $TARGET - ($i)}")
  done
  if [ "$fmt" = wav ]; then
    ff -i "$mid" -ac 1 -ar 44100 -c:a pcm_s16le "$OUT/$id.wav"
  else
    ff -i "$mid" -ac 2 -ar 44100 -c:a vorbis -strict -2 -q:a 3 "$OUT/$id.ogg"
  fi
}

# loop ID SRC START LEN XFADE [FILTERS] [channels]: a seamless loop of LEN seconds.
# body = src[S+X, S+X+LEN], head = src[S, S+X]; the body's last X seconds crossfade into the
# head, so the loop's last sample is src[S+X-] and its first is src[S+X]: continuous.
loop() {
  local id=$1 src=$2 s=$3 len=$4 x=$5 extra=${6:-anull} ch=${7:-1}
  local total; total=$(awk "BEGIN{printf \"%.4f\", $len + $x}")
  ff -ss "$s" -t "$total" -i "$src" -filter_complex \
    "[0:a]aformat=sample_fmts=flt,aresample=44100,$extra,asplit[a][b];\
     [a]atrim=start=$x:duration=$len,asetpts=PTS-STARTPTS[body];\
     [b]atrim=start=0:duration=$x,asetpts=PTS-STARTPTS[head];\
     [body][head]acrossfade=d=$x:c1=qsin:c2=qsin[o]" \
    -map "[o]" -ac "$ch" -ar 44100 -c:a pcm_f32le "$TMP/$id.pre.wav"
}

SOFT="lowpass=f=9000"          # takes the edge off bright recordings
SOFTER="lowpass=f=6500,lowpass=f=9000"

# ── the coffee machine ──
cut sfx_machine_hiss   "$FS/868483__sokworks__steaming-milk-low.wav" 4.0 1.4 0.12 0.55 "highpass=f=120,$SOFTER"
finish sfx_machine_hiss wav
cut sfx_machine_purge  "$FS/824466__propolisher__coffee-machine-cup-rinse-boiler-hum-steam-wand-purge.wav" 51.5 1.9 0.01 0.7 "highpass=f=80,$SOFTER"
finish sfx_machine_purge wav
cut sfx_machine_sputter "$FS/868481__sokworks__steaming-milk-finish.wav" 0.0 1.3 0.01 0.35 "$SOFT"
finish sfx_machine_sputter wav
cut sfx_shot_pull      "$FS/868497__sokworks__espresso-machine-empty-shot-pull.wav" 0.0 1.8 0.005 0.6 "highpass=f=60,$SOFT"
finish sfx_shot_pull wav
# the group head: a knock on the knock box, then the end of a blank shot (the pump stops)
ff -i "$FS/868472__sokworks__knock-box-2.wav" -ss 4.4 -t 1.25 -i "$FS/868497__sokworks__espresso-machine-empty-shot-pull.wav" -filter_complex \
  "[0:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,afade=t=out:st=0.15:d=0.04[k];\
   [1:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,volume=6dB,afade=t=in:d=0.15,afade=t=out:st=0.95:d=0.3,adelay=280[f];\
   [k][f]amix=inputs=2:normalize=0:duration=longest,highpass=f=60,$SOFT[o]" -map "[o]" -ac 1 -ar 44100 -c:a pcm_f32le "$TMP/sfx_group_clean.pre.wav"
finish sfx_group_clean wav
cut sfx_knock_box      "$FS/868472__sokworks__knock-box-2.wav" 0.0 0.1875 0.002 0.05 "$SOFT"
finish sfx_knock_box wav

# ── the stove ──
loop amb_soup_simmer   "$FS/124584__alienistcog__simmering-sauce.aiff" 60.0 14.0 1.0 "highpass=f=70,$SOFT" 2
finish amb_soup_simmer ogg
cut sfx_pot_lid        "$K/metalPot1.ogg" 0.0 1.2 0.002 0.5 "$SOFTER"
finish sfx_pot_lid wav
cut sfx_pot_stir       "$K/metalPot2.ogg" 0.0 0.9 0.002 0.4 "$SOFTER"
finish sfx_pot_stir wav

# ── the oven ──
cut sfx_oven_ding      "$FS/149506__dland__kitchen-timer-done.wav" 2.06 2.1 0.004 1.0 "highpass=f=200,$SOFT"
finish sfx_oven_ding wav
# open (0.05–0.8 s) and close (3.0–3.65 s) of one take, joined
ff -ss 0.05 -t 0.75 -i "$FS/503477__bolkmar__oven-door-open-and-close.wav" -ss 2.95 -t 0.7 -i "$FS/503477__bolkmar__oven-door-open-and-close.wav" -filter_complex \
  "[0:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,afade=t=in:d=0.01[a];\
   [1:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,afade=t=out:st=0.45:d=0.25[b];\
   [a][b]acrossfade=d=0.08:c1=qsin:c2=qsin,highpass=f=60,$SOFT[o]" -map "[o]" -ac 1 -ar 44100 -c:a pcm_f32le "$TMP/sfx_oven_door.pre.wav"
finish sfx_oven_door wav

# ── the fridge ──
cut sfx_fridge_door    "$FS/643537__the_isot_is_back__fridge-door-open-close.wav" 0.08 1.77 0.01 0.3 "highpass=f=50,$SOFT"
finish sfx_fridge_door wav
cut sfx_bottle_clink   "$FS/816964__luisa_sanchez__clinking-of-bottles.wav" 0.0 0.6 0.003 0.12 "lowpass=f=5000,lowpass=f=7000"
finish sfx_bottle_clink wav

# ── closing ──
cut sfx_till_coins     "$K/handleCoins.ogg" 0.0 0.845 0.003 0.2 "$SOFT"
finish sfx_till_coins wav
cut sfx_trash_bag      "$FS/154371__semccab__trashbag.wav" 4.6 1.5 0.05 0.4 "highpass=f=250,highpass=f=250,$SOFT"
finish sfx_trash_bag wav
# a wooden door: open, a breath, close
ff -i "$K/doorOpen_1.ogg" -i "$K/doorClose_2.ogg" -filter_complex \
  "[0:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,atrim=duration=0.85,afade=t=out:st=0.6:d=0.25[a];\
   [1:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,adelay=700,afade=t=out:st=1.2:d=0.1[b];\
   [a][b]amix=inputs=2:normalize=0:duration=longest,$SOFT[o]" -map "[o]" -ac 1 -ar 44100 -c:a pcm_f32le "$TMP/sfx_door_outside.pre.wav"
finish sfx_door_outside wav
cut sfx_lamp_switch    "$FS/870289__funkysandwich__light_switch10.wav" 0.0 0.15 0.001 0.04 "$SOFT"
finish sfx_lamp_switch wav

# ── the walk home ──
cut sfx_keys_lock      "$FS/615287__nickrandomsounds__unlocking-locking-door-and-keys-rattling.wav" 2.45 2.2 0.02 0.5 "highpass=f=80,$SOFTER"
finish sfx_keys_lock wav
cut sfx_pigeons        "$FS/689998__kinoton__pigeon-flies-away-flapping-wings.wav" 0.35 2.3 0.05 0.8 "highpass=f=90,$SOFT"
finish sfx_pigeons wav
cut sfx_tram_bell      "$FS/497561__16hpanskawillheimova_sara__11-3-tram-bell.wav" 0.22 1.6 0.004 0.4 "highpass=f=150,$SOFTER"
finish sfx_tram_bell wav
cut sfx_tram_pass      "$FS/497560__16hpanskawillheimova_sara__15_tram-passing-by.wav" 8.0 7.0 1.2 2.5 "highpass=f=40,$SOFT"
finish sfx_tram_pass ogg
# wet steps: the Freesound wet-sidewalk take is buried in traffic, so the steps are Kenney's
# concrete steps (CC0) at a slow stroll, each with a soft splash of filtered noise (made here)
args=(); fc=""; n=0
for f in 0 2 1 4 3 1 0 3 2 4 1 2 0 4 3 2; do
  args+=(-i "$KI/footstep_concrete_00$f.ogg")
  t=$((20 + n * 560))
  fc+="[$n:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,lowpass=f=4500,volume=$(( n % 2 ? -2 : 0 ))dB,adelay=$t,apad=whole_dur=8.96[s$n];"
  n=$((n+1))
done
# the splash: a short burst of filtered noise on every step (made here, not recorded)
fc+="aevalsrc='(random(0)*2-1)*if(between(mod(t-0.025,0.56),0,0.09),exp(-mod(t-0.025,0.56)/0.02),0)*(0.75+0.25*sin(t*2.3))':s=44100:d=8.96,highpass=f=1500,lowpass=f=5500,volume=0.35[wet];"
fc+="$(for i in $(seq 0 15); do printf "[s%d]" "$i"; done)[wet]amix=inputs=17:normalize=0:duration=first[o]"
ff "${args[@]}" -filter_complex "$fc" -map "[o]" -t 8.96 -ac 1 -ar 44100 -c:a pcm_f32le "$TMP/amb_wet_steps.pre.wav"
finish amb_wet_steps ogg

# ── the back step ──
cut sfx_door_creak     "$K/creak1.ogg" 0.0 0.66 0.005 0.2 "$SOFT"
finish sfx_door_creak wav
loop amb_rain_awning   "$FS/454130__kyles__rain-light-on-awning-close-drops-on-fabric.flac" 20.0 15.0 1.0 "highpass=f=80,$SOFT" 2
finish amb_rain_awning ogg
cut sfx_phone_buzz     "$FS/637346__kyles__cell-phone-smartphone-vibrate-single.flac" 1.15 1.7 0.01 0.1 "highpass=f=60,lowpass=f=6000"
finish sfx_phone_buzz wav
loop amb_cat_purr      "$FS/553962__clareboots__cat-purr.wav" 62.5 10.5 1.0 "highpass=f=40,lowpass=f=5000" 1
finish amb_cat_purr ogg
cut sfx_cat_meow       "$FS/866536__sokworks__meow-gentle.wav" 0.0 1.0 0.01 0.3 "highpass=f=100,$SOFT"
finish sfx_cat_meow wav

# ── the storm ──
loop amb_storm         "$FS/331527__davestalker__wind-howling-thru-window-crack.mp3" 6.5 16.0 1.5 "highpass=f=70,lowpass=f=7000" 1
finish amb_storm ogg

# ── books, blankets, chalk, the polaroid ──
cut sfx_book_open      "$K/bookOpen.ogg" 0.0 0.1537 0.002 0.04 "$SOFT"
finish sfx_book_open wav
cut sfx_page_flip      "$K/bookFlip1.ogg" 0.0 0.768 0.003 0.25 "$SOFT"
finish sfx_page_flip wav
cut sfx_blanket        "$K/cloth1.ogg" 0.0 0.66 0.01 0.25 "$SOFTER"
finish sfx_blanket wav
cut sfx_chalk          "$FS/447925__breviceps__writedraw-with-chalk-on-board.wav" 0.0 0.476 0.004 0.12 "highpass=f=200,$SOFTER"
finish sfx_chalk wav
cut photo_shutter      "$FS/151800__jamesmontour__polaroid-spirit-600-shutter-wav.wav" 0.36 0.7 0.004 0.3 "highpass=f=60,$SOFT"
finish photo_shutter wav
# the print comes out: the motor starts (2.05–2.9 s) and stops (7.4–8.15 s), joined
ff -ss 2.05 -t 0.85 -i "$FS/616967__trp__210506-polaroid-camera-click-whirr.wav" -ss 7.4 -t 0.75 -i "$FS/616967__trp__210506-polaroid-camera-click-whirr.wav" -filter_complex \
  "[0:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,afade=t=in:d=0.01[a];\
   [1:a]aformat=sample_fmts=flt,aresample=44100,pan=mono|c0=0.5*c0+0.5*c1,afade=t=out:st=0.6:d=0.15[b];\
   [a][b]acrossfade=d=0.12:c1=qsin:c2=qsin,highpass=f=80,$SOFT[o]" -map "[o]" -ac 1 -ar 44100 -c:a pcm_f32le "$TMP/photo_eject.pre.wav"
finish photo_eject wav

echo done
