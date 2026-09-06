# 기여 안내

이 저장소는 Linux에서 KiwiTalk을 다시 사용할 수 있도록 복구하는 포크입니다. 로그인·기기 등록, 채팅 동기화, Linux 호환성 개선을 우선하고 있습니다.

개발 환경과 실행 명령은 [README](./README.md#개발-환경에서-실행), 코드 구조는 [ARCHITECTURE.md](./ARCHITECTURE.md)를 참고해 주세요. 참여할 때는 [행동 강령](./CODE_OF_CONDUCT.md)을 지켜 주세요.

## 이슈

[이 저장소의 Issues](https://github.com/dlwltkd/kiwitalk/issues)에 아래 정보를 함께 적어 주세요.

- OS·배포판, 데스크톱 환경, 사용한 커밋
- 재현 절차와 기대한 동작, 실제 결과
- 관련 로그와 오류 메시지

비밀번호, 인증 토큰, 계정 식별 정보와 개인 대화 내용은 제거해 주세요. 범위가 큰 변경은 이슈에서 먼저 논의해 주세요.

## Pull Request

1. 이 저장소를 포크하고 `main`에서 작업 브랜치를 만듭니다.
2. 변경과 관련된 검사를 실행합니다. 프런트엔드는 `pnpm run frontend:typecheck`, Rust는 `cargo check --workspace --locked`로 확인하고, 동작을 바꿨다면 관련 테스트도 실행해 주세요.
3. 커밋 메시지는 `fix:`, `feat:`, `docs:`처럼 변경 목적을 드러내는 형식으로 작성합니다.
4. [이 저장소에 PR](https://github.com/dlwltkd/kiwitalk/compare)을 열고 대상(base) 브랜치를 `main`으로 선택합니다. 변경 이유, 확인한 환경과 결과, 남은 문제를 적어 주세요.

UI 변경에는 스크린샷을 첨부해 주세요. 실행하지 못한 검사는 그 이유를 함께 적어 주세요.

## 라이선스

기여한 코드에도 [Apache License 2.0](./LICENSE-APACHE)을 적용합니다. 기존 저작권·출처 표기를 유지하고, 외부 코드나 자산을 포함할 때는 출처와 라이선스를 함께 기록해 주세요. 원본과 이 포크의 출처는 [NOTICE](./NOTICE)에 정리되어 있습니다.
