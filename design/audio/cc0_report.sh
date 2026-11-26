#!/bin/bash
# Table of the CC0 sounds (assets/audio/cc0/): id, duration, true peak, integrated LUFS and the
# distance to the -24 LUFS target; for loops the seam (RMS of the last / first 10 ms, and the
# largest sample step across the wrap against the largest step in the file: no click).
cd "$(dirname "$0")/../.."
FF=${FFMPEG:-ffmpeg}
OUT=assets/audio/cc0
TARGET=-24
rms() { # FILE START DUR -> RMS dB (mono mix)
  $FF -hide_banner -nostats -ss "$2" -t "$3" -i "$1" -af "pan=mono|c0=0.5*c0+0.5*c1,astats=measure_overall=RMS_level:measure_perchannel=none" -f null - 2>&1 \
    | awk '/RMS level dB/{print $NF}' | tail -1
}
jump() { # FILE -> peak of the first difference (dB)
  $FF -hide_banner -nostats -i "$1" -af "pan=mono|c0=0.5*c0+0.5*c1,aderivative,astats=measure_overall=Peak_level:measure_perchannel=none" -f null - 2>&1 \
    | awk '/Peak level dB/{print $NF}' | tail -1
}
printf "%-22s %6s %7s %7s %6s  %s\n" id dur peak LUFS dLU seam
for f in "$OUT"/*; do
  id=$(basename "$f"); id=${id%.*}
  d=$(${FFPROBE:-ffprobe} -v error -show_entries format=duration -of csv=p=0 "$f" | tr -d "[:space:]")
  read -r i p < <($FF -hide_banner -nostats -i "$f" -af "apad=pad_dur=0.5,ebur128=peak=true" -f null - 2>&1 \
    | awk '/Summary/{s=1} s&&/I:/{i=$2} s&&/Peak:/{p=$2} END{print i, p}')
  seam=""
  if [[ $id == amb_* ]]; then
    tmp=$(mktemp -t seam).wav
    $FF -v error -y -i "$f" -c:a pcm_f32le "$tmp"
    end=$(awk "BEGIN{printf \"%.4f\", $d - 0.010}")
    a=$(rms "$tmp" "$end" 0.010); b=$(rms "$tmp" 0 0.010)
    # the wrap itself: the largest sample-to-sample step across the seam (last 20 ms + first
    # 20 ms) against the largest step anywhere in the file; a click would stand out
    w0=$(awk "BEGIN{printf \"%.4f\", $d - 0.020}")
    $FF -v error -y -ss "$w0" -i "$tmp" -t 0.020 -i "$tmp" \
      -filter_complex "[0:a][1:a]concat=n=2:v=0:a=1" -c:a pcm_f32le "${tmp%.wav}_w.wav"
    jw=$(jump "${tmp%.wav}_w.wav"); ja=$(jump "$tmp")
    seam=$(printf "last10ms %.1f / first10ms %.1f dB RMS; max step at seam %s dB vs file %s dB" "$a" "$b" "$jw" "$ja")
    rm -f "$tmp" "${tmp%.wav}_w.wav"
  fi
  dl=$(awk "BEGIN{printf \"%+.1f\", $i - ($TARGET)}")
  printf "%-22s %6.2f %7s %7s %6s  %s\n" "$id" "$d" "$p" "$i" "$dl" "$seam"
done
du -ch "$OUT"/* | tail -1
