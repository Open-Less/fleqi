#!/bin/bash
set -euo pipefail
FLEQI_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FLEQI_BUILD="$FLEQI_ROOT/.build/engines"
FLEQI_BIN="$FLEQI_ROOT/src-tauri/runtime-resources/bin"
FLEQI_LICENSES="$FLEQI_ROOT/src-tauri/runtime-resources/licenses"
mkdir -p "$FLEQI_BUILD" "$FLEQI_BIN" "$FLEQI_LICENSES"
cd "$FLEQI_BUILD"
# 交叉平台的编译验证与快速 push 构建用这个开关跳过静态引擎编译：产物不具备
# 媒体转码与 PDF 能力，只能用于验证代码可构建，不能用于分发。发版流程必须完整构建。
if [[ "${FLEQI_SKIP_ENGINES:-0}" == "1" ]]; then
  echo 'FLEQI_SKIP_ENGINES=1：跳过 FFmpeg/qpdf 构建，本次产出的应用没有媒体与 PDF 能力。'
  exit 0
fi
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'Build platform-specific FFmpeg/qpdf and place them in runtime-resources/bin before packaging.'
  exit 1
fi
# Static codec dependencies are build inputs only. The resulting applications
# depend on Apple system libraries, never on a user Homebrew installation.
if [[ ! -f ffmpeg-8.0.1.tar.xz ]]; then curl -fL --retry 2 https://ffmpeg.org/releases/ffmpeg-8.0.1.tar.xz -o ffmpeg-8.0.1.tar.xz; fi
if [[ ! -d ffmpeg-8.0.1 ]]; then tar -xf ffmpeg-8.0.1.tar.xz; fi
if ! tar -tzf qpdf-12.4.1.tar.gz >/dev/null 2>&1; then curl -fsSL --retry 3 --retry-all-errors --continue-at - https://github.com/qpdf/qpdf/releases/download/v12.4.1/qpdf-12.4.1.tar.gz -o qpdf-12.4.1.tar.gz; fi
if [[ ! -d qpdf-12.4.1 ]]; then tar -xzf qpdf-12.4.1.tar.gz; fi
if [[ ! -f "$FLEQI_BUILD/source/lame-4.0.tar.gz" ]]; then
  mkdir -p "$FLEQI_BUILD/source"
  curl -fsSL --retry 3 --retry-all-errors https://downloads.sourceforge.net/project/lame/lame/4.0/lame-4.0.tar.gz -o "$FLEQI_BUILD/source/lame-4.0.tar.gz"
fi
if [[ ! -d "$FLEQI_BUILD/source/lame-4.0" ]]; then tar -xzf "$FLEQI_BUILD/source/lame-4.0.tar.gz" -C "$FLEQI_BUILD/source"; fi
if [[ ! -f "$FLEQI_BUILD/source/lame-4.0/libmp3lame/.libs/libmp3lame.a" ]]; then
  cd "$FLEQI_BUILD/source/lame-4.0"
  CFLAGS='-O2 -mmacosx-version-min=15.0' LDFLAGS='-mmacosx-version-min=15.0' ./configure --disable-shared --enable-static --disable-decoder --disable-frontend > "$FLEQI_BUILD/lame-configure.log" 2>&1
  make -j8 > "$FLEQI_BUILD/lame-build.log" 2>&1
fi
if [[ ! -f "$FLEQI_BUILD/ffmpeg-macos15.ready" ]]; then
  cd "$FLEQI_BUILD/ffmpeg-8.0.1"
  mkdir -p ../static
  mkdir -p ../include/lame
  install -m 644 ../source/lame-4.0/libmp3lame/.libs/libmp3lame.a ../static/libmp3lame.a
  install -m 644 ../source/lame-4.0/include/lame.h ../include/lame/lame.h
  ./configure --prefix="$FLEQI_BUILD/install" --disable-shared --enable-static --disable-debug --disable-doc --disable-ffplay --disable-autodetect --enable-videotoolbox --enable-audiotoolbox --enable-libmp3lame --extra-cflags='-I../include -mmacosx-version-min=15.0' --extra-ldflags='-L../static -mmacosx-version-min=15.0' --disable-network > "$FLEQI_BUILD/ffmpeg-configure.log" 2>&1
  make -j8 > "$FLEQI_BUILD/ffmpeg-build.log" 2>&1
  cp ffmpeg ffprobe "$FLEQI_BIN/"
  cp COPYING.LGPLv2.1 "$FLEQI_LICENSES/FFmpeg-LICENSE"
  cp "$FLEQI_BUILD/source/lame-4.0/COPYING" "$FLEQI_LICENSES/LAME-LICENSE"
  touch "$FLEQI_BUILD/ffmpeg-macos15.ready"
fi
if [[ ! -f "$FLEQI_BUILD/source/libjpeg-turbo-3.2.0.tar.gz" ]]; then curl -fsSL --retry 3 --retry-all-errors https://github.com/libjpeg-turbo/libjpeg-turbo/releases/download/3.2.0/libjpeg-turbo-3.2.0.tar.gz -o "$FLEQI_BUILD/source/libjpeg-turbo-3.2.0.tar.gz"; fi
if [[ ! -d "$FLEQI_BUILD/source/libjpeg-turbo-3.2.0" ]]; then tar -xzf "$FLEQI_BUILD/source/libjpeg-turbo-3.2.0.tar.gz" -C "$FLEQI_BUILD/source"; fi
if [[ ! -f "$FLEQI_BUILD/jpeg-build/libjpeg.a" ]]; then
  cmake -S "$FLEQI_BUILD/source/libjpeg-turbo-3.2.0" -B "$FLEQI_BUILD/jpeg-build" -DENABLE_SHARED=OFF -DENABLE_STATIC=ON -DWITH_TURBOJPEG=OFF -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_DEPLOYMENT_TARGET=15.0 > "$FLEQI_BUILD/jpeg-configure.log" 2>&1
  cmake --build "$FLEQI_BUILD/jpeg-build" --target jpeg-static --parallel 8 > "$FLEQI_BUILD/jpeg-build.log" 2>&1
fi
if [[ ! -f "$FLEQI_BUILD/qpdf-macos15.ready" ]]; then
  cmake -S "$FLEQI_BUILD/qpdf-12.4.1" -B "$FLEQI_BUILD/qpdf-static" -DBUILD_SHARED_LIBS=OFF -DBUILD_STATIC_LIBS=ON -DBUILD_DOC=OFF -DINSTALL_EXAMPLES=OFF -DUSE_IMPLICIT_CRYPTO=OFF -DREQUIRE_CRYPTO_NATIVE=ON -DDEFAULT_CRYPTO=native -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_DEPLOYMENT_TARGET=15.0 -DPKG_CONFIG_EXECUTABLE=/usr/bin/false -DLIBJPEG_LIB_PATH="$FLEQI_BUILD/jpeg-build/libjpeg.a" -DLIBJPEG_H_PATH="$FLEQI_BUILD/source/libjpeg-turbo-3.2.0/src" -DCMAKE_CXX_FLAGS="-I\"$FLEQI_BUILD/jpeg-build\"" > "$FLEQI_BUILD/qpdf-configure.log" 2>&1
  cmake --build "$FLEQI_BUILD/qpdf-static" --target qpdf --parallel 8 > "$FLEQI_BUILD/qpdf-build.log" 2>&1
  cp "$FLEQI_BUILD/qpdf-static/qpdf/qpdf" "$FLEQI_BIN/qpdf"
  cp "$FLEQI_BUILD/qpdf-12.4.1/LICENSE.txt" "$FLEQI_LICENSES/qpdf-LICENSE"
  cp "$FLEQI_BUILD/source/libjpeg-turbo-3.2.0/LICENSE.md" "$FLEQI_LICENSES/libjpeg-turbo-LICENSE"
  touch "$FLEQI_BUILD/qpdf-macos15.ready"
fi
otool -L "$FLEQI_BIN/ffmpeg" "$FLEQI_BIN/ffprobe" "$FLEQI_BIN/qpdf"
echo 'Media and PDF engines prepared.'
