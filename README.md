# KiwiTalk

Linux에서 다시 쓰기 위해 되살리는 비공식 카카오톡 클라이언트.

## 다시 시작하는 이유

최근 약 2년간 KiwiTalk을 제대로 사용하기 어려웠던 경험에서 시작했습니다. Linux에서 로그인하고 대화를 주고받는 기본 기능부터 복구해, 다시 사용할 수 있는 클라이언트로 만드는 것이 목표입니다.

[원본 KiwiTalk](https://github.com/KiwiTalk/KiwiTalk)에서 출발한 독립 포크입니다. 원본의 개발 내역과 기존 안내는 해당 저장소에서 확인해 주세요.

## 현재 상태

`main` 브랜치에서 Linux를 우선 대상으로 개발 중입니다. SolidJS·TypeScript로 화면을 구성하고, Rust·Tauri 2로 데스크톱 앱과 통신을 처리합니다.

| 영역     | 현재 구현                                          |
| -------- | -------------------------------------------------- |
| 데스크톱 | Tauri 2 전환, Linux 창·트레이 처리                 |
| 인증     | 로그인, Android 보조 기기 인증·등록, 자동 로그인    |
| 목록     | 친구·채팅방 조회, 채팅방 필터                      |
| 대화     | 텍스트 메시지 송수신, 이전 대화 조회와 로컬 동기화 |
| 화면     | 터미널 스타일 UI, 키보드 탐색                      |

설정 화면은 아직 미완성입니다. 실제 계정·환경별 호환성과 Windows·macOS 지원은 추가 검증이 필요합니다.

KiwiTalk는 Android 보조 기기로 접속합니다. 기존 태블릿의 카카오톡 접속이 종료될 수 있으므로 태블릿과의 동시 사용은 지원하지 않습니다. 이전 대화는 서버가 반환한 기록과 이 PC에 저장된 기록을 표시하며, 다른 기기에만 저장된 오래된 대화까지 모두 복원하지는 못합니다.

> Kakao Corp.가 제작하거나 승인한 클라이언트가 아닙니다. 비공식 연구·개발용 프로젝트로, 서버 변경이나 정책에 따라 동작하지 않거나 이용이 제한될 수 있습니다. 사용에 따른 위험과 책임은 사용자에게 있습니다.

## 개발 환경에서 실행

도구 버전은 [.mise.toml](./.mise.toml)에 고정되어 있습니다.

- Node.js 20.20.2
- pnpm 9.15.9
- Rust 1.98.0
- [Tauri 2의 OS별 시스템 의존성](https://v2.tauri.app/start/prerequisites/)과 GUI 실행 환경

Linux에서는 GTK 3, WebKitGTK 4.1 등 네이티브 라이브러리가 필요합니다. 배포판별 설치 방법은 위 Tauri 문서를 참고해 주세요.

```sh
git clone --branch main https://github.com/dlwltkd/kiwitalk.git
cd kiwitalk
pnpm install --frozen-lockfile
pnpm run dev
```

| 명령                          | 용도                    |
| ----------------------------- | ----------------------- |
| `pnpm run dev`                | 네이티브 앱 개발 모드   |
| `pnpm run build`              | Linux 실행 파일과 DEB·RPM 패키지 빌드 |
| `pnpm run frontend:dev`       | 프런트엔드 개발 서버    |
| `pnpm run frontend:typecheck` | 프런트엔드 타입 검사    |
| `pnpm run frontend:test`      | 대화 목록·페이지 이동 회귀 테스트 |
| `pnpm run storybook`          | UI 컴포넌트 개발        |

로그인과 채팅에는 네이티브 백엔드가 필요하므로 앱을 사용하려면 `pnpm run dev`로 실행해야 합니다.

## 기여

버그와 기능 제안은 [이 저장소의 Issues](https://github.com/dlwltkd/kiwitalk/issues)에 남겨 주세요. OS·데스크톱 환경, 사용한 커밋, 재현 절차를 함께 적으면 확인에 도움이 됩니다. 로그에서는 비밀번호, 인증 토큰, 개인 대화 내용을 제거해 주세요.

개발 환경과 PR 안내는 [CONTRIBUTING.md](./CONTRIBUTING.md), 코드 구조는 [ARCHITECTURE.md](./ARCHITECTURE.md)를 참고해 주세요.

현재 Android 호환성 구현과 검증 범위는 [프로토콜 조사 기록](./docs/protocol/android-26.7.2.md)에 정리되어 있습니다.

## 라이선스와 출처

원본 KiwiTalk과 이 포크의 코드는 **Apache License 2.0**으로 배포합니다. 원본 기여자의 저작권·출처 표기를 유지하며, 이 포크의 수정분에도 같은 라이선스를 적용합니다.

- [LICENSE-APACHE](./LICENSE-APACHE): Apache License 2.0 전문
- [NOTICE](./NOTICE): 원본 프로젝트, 기여자, 포크의 주요 변경 내역
- [Pretendard 라이선스](./frontend/public/fonts/OFL.txt): 포함된 폰트에 적용되는 SIL Open Font License 1.1
- [futures-loco-protocol 라이선스](./vendor/futures-loco-protocol/LICENSE): 수정해 포함한 전송 라이브러리의 MIT 라이선스. [수정 내역](./vendor/futures-loco-protocol/PATCHES.md)

외부 의존성과 자산은 각자의 라이선스를 따릅니다. 카카오톡 및 관련 상표의 권리는 각 권리자에게 있습니다.
