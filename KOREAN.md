# DeckCraft 한국어 패치

[storytold/deckcraft](https://github.com/storytold/deckcraft)에 한국어 인터페이스를 더한 비공식 포크입니다.

## 받기
[Releases](../../releases)에서 `DeckCraft-ko-windows-x64.zip`을 받아 압축을 풀고 `DeckCraft.exe`를 실행합니다.
서명이 없는 빌드라 처음 실행할 때 Windows가 경고하면 **추가 정보 → 실행**을 누르세요.
영어로 쓰려면 환경 변수 `DECKCRAFT_LANG=en`을 설정합니다.

## 바뀐 곳
- `vendor/egui`: storytold/egui(고정 rev)에 번역 훅(`set_translator`)을 넣은 사본. 위젯·툴팁·창 제목·직접 그린
  글자가 모두 이 훅을 거칩니다.
- `crates/ui-egui/src/i18n_ko.rs`, `crates/ui-egui/i18n/ko.tsv`: 한국어 번역표와 조회(정확히 일치, `{…}` 템플릿, `…`).
- 리본 단추·탭·상태 표시줄은 글자 폭을 미리 재므로 번역된 글자로 재도록 수정.
- 한국어 모드에서는 시작할 때 공식 업데이트 확인을 하지 않습니다(영어판으로 덮어쓰지 않도록).
- `.github/workflows/korean-windows.yml`: `main`에 푸시하면 Windows 빌드를 만들어 릴리스로 올립니다.

## 번역 고치기
`crates/ui-egui/i18n/ko.tsv`에 `영어<TAB>한국어` 줄을 추가하거나 고친 뒤 `main`에 푸시하면 새 빌드가 나옵니다.
