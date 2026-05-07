CREATE OR REPLACE FUNCTION apply_push_batch(batch_json jsonb, writer_epoch integer)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  row_count integer := 0;
BEGIN
  IF NOT l4_is_any_role(ARRAY['daemon', 'admin']) THEN
    RAISE EXCEPTION 'apply_push_batch requires daemon role'
      USING ERRCODE = '42501';
  END IF;

  IF jsonb_typeof(batch_json -> 'rows') = 'array' THEN
    row_count := jsonb_array_length(batch_json -> 'rows');
  END IF;

  RETURN jsonb_build_object(
    'status', 'not_implemented',
    'accepted', false,
    'row_count', row_count,
    'writer_epoch', writer_epoch
  );
END;
$$;

CREATE OR REPLACE FUNCTION update_operator_decision(
  process_id text,
  disposition text,
  base_revision bigint,
  command_id text
)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
  IF NOT l4_is_any_role(ARRAY['operator', 'admin']) THEN
    RAISE EXCEPTION 'update_operator_decision requires operator role'
      USING ERRCODE = '42501';
  END IF;

  IF disposition NOT IN ('pending', 'watching', 'approved', 'rejected') THEN
    RAISE EXCEPTION 'invalid operator disposition: %', disposition
      USING ERRCODE = '22023';
  END IF;

  RETURN jsonb_build_object(
    'status', 'not_implemented',
    'accepted', false,
    'process_id', process_id,
    'base_revision', base_revision,
    'command_id', command_id
  );
END;
$$;

GRANT EXECUTE ON FUNCTION apply_push_batch(jsonb, integer) TO authenticated, service_role;
GRANT EXECUTE ON FUNCTION update_operator_decision(text, text, bigint, text) TO authenticated, service_role;
