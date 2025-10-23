# MTConnect Client (Rust)

북미 제조 표준 규격 MTConnect 프로토콜을 사용하는 공작기계 데이터 수집 클라이언트

## 특징

- **실시간 데이터 수집**: 1초 간격으로 공작기계 상태 모니터링
- **타입 안전**: Rust의 강력한 타입 시스템 활용
- **비동기 처리**: tokio 런타임으로 효율적인 I/O
- **자동 재연결**: 네트워크 에러 시 자동 재시도
- **CSV 로깅**: 수집 데이터 자동 저장

## 수집 데이터

- **스핀들 정보**: Load, Speed, Override
- **이송 정보**: Feed Override, Feedrate
- **생산 정보**: Part Count, Program Name
- **상태 정보**: Execution, Controller Mode, Emergency Stop, Alarm

## 설치 및 실행

### 필요 요구사항

- Rust 1.70 이상
- MTConnect Agent (예: http://agent.mtconnect.org)

### 빌드

```bash
cargo build --release
```

### 실행

기본 Agent URL 사용:
```bash
cargo run
```

커스텀 Agent URL 지정:
```bash
MTCONNECT_URL=http://your-agent:5000 cargo run
```

### 릴리즈 빌드 실행

```bash
./target/release/mtconnect-client
```

## 환경 변수

- `MTCONNECT_URL`: MTConnect Agent URL (기본값: http://agent.mtconnect.org)

## 출력 예시

```
MTConnect Client Starting...
Agent URL: http://agent.mtconnect.org

Initializing collector...
✓ Connected to device: OKUMA.Lathe
✓ Initial sequence: 12345

=== Starting Continuous Collection ===
Press Ctrl+C to stop

[2025-10-23 10:15:30] OKUMA.Lathe
  🟢 Status: ACTIVE
  📄 Program: O0001
  ⚙️  Spindle Load: 45.2%
  🔄 Spindle Speed: 1200 RPM
  ➡️  Feed Override: 100%
  📊 Feedrate: 150.5 mm/min
  📦 Parts Produced: 42
```

## 데이터 로깅

프로그램 실행 시 자동으로 `machine_data.csv` 파일에 데이터가 저장됩니다.

CSV 형식:
```csv
timestamp,device_name,program_name,execution,spindle_load,feed_override,part_count,alarm
2025-10-23T10:15:30Z,OKUMA.Lathe,O0001,ACTIVE,45.2,100,42,
```

## 프로젝트 구조

```
mtconnect-client/
├── Cargo.toml              # 의존성 설정
├── src/
│   ├── main.rs            # 메인 실행 파일
│   ├── lib.rs             # 라이브러리 모듈
│   ├── machine_data.rs    # 데이터 모델 정의
│   ├── parser.rs          # XML 파싱 구조
│   ├── collector.rs       # 데이터 수집 로직
│   └── storage.rs         # 데이터 저장 로직
└── README.md
```

## 라이브러리로 사용

```rust
use mtconnect_client::{MachineDataCollector, ExecutionState};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut collector = MachineDataCollector::new("http://agent.mtconnect.org".to_string());
    
    collector.initialize().await?;
    
    collector.collect_continuous(Duration::from_secs(1), |snapshot| {
        if snapshot.execution == Some(ExecutionState::ACTIVE) {
            println!("Machine is running!");
        }
    }).await?;
    
    Ok(())
}
```

## MTConnect 프로토콜

MTConnect는 제조 장비의 데이터를 표준화된 HTTP/REST API로 제공합니다.

주요 엔드포인트:
- `/probe`: 장비 메타데이터
- `/current`: 현재 상태 스냅샷
- `/sample`: 시간 범위 데이터

자세한 내용: https://www.mtconnect.org/

## 기술 스택

- **tokio**: 비동기 런타임
- **reqwest**: HTTP 클라이언트
- **quick-xml**: XML 파싱
- **serde**: 직렬화/역직렬화
- **chrono**: 날짜/시간 처리

## 라이센스

MIT License

## 작성자

Rust + Scala/Akka 개발자를 위한 MTConnect 클라이언트
