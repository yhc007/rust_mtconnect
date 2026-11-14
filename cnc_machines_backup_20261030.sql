-- cnc_machines 포트 매핑 복원 스크립트 (2026-10-30 백업)
--
-- 커밋 243a42a 가 코드 쪽 5006/5007 라벨을 교정했으나 cnc_machines 테이블은
-- 옛 매핑으로 남아 있었다. 파라미터 시트(SM0160)와 Agent probe 의 uuid 가
-- 모두 아래 매핑을 가리키므로 2026-10-30 에 테이블도 맞췄다.
--
--   포트  Agent uuid           기종          교정 후 nc_id
--   5006  MT600_SN306319       MT600  2호기  MNT600-2
--   5007  MT600S_SN306321      MT600S 1호기  MNT600S-1
--
-- 이 파일은 그 변경 "이전" 값으로 되돌리는 문장이다. cnc_machines 는
-- elfin-focas-apapter 가 스키마를 소유하고 fanuc_focas_rust·AAS 가 읽는
-- 공유 테이블이므로, 그쪽 대시보드가 옛 라벨에 맞춰 운용되고 있었다면
-- 이 스크립트로 즉시 되돌릴 수 있다.
--
-- 주의: MTConnect 수집기는 이 테이블을 읽지 않는다 (mtconnect.rs 의
-- 하드코딩 목록 사용). 되돌려도 수집 데이터의 nc_id 는 바뀌지 않는다.

UPDATE cnc_machines SET nc_port=5006 WHERE id='MNT600S-1';  -- 복원용 (2026-10-30 백업)
UPDATE cnc_machines SET nc_port=5007 WHERE id='MNT600-2';  -- 복원용 (2026-10-30 백업)
