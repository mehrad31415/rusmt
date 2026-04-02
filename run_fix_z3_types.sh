#!/bin/bash
LOG_FILE="z3_fix_types.log"

echo "$(date): Starting z3-sys type fix task" | tee -a "$LOG_FILE"

MAX_RETRIES=5
RETRY_COUNT=0

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    echo "$(date): Attempt $((RETRY_COUNT + 1))/$MAX_RETRIES" | tee -a "$LOG_FILE"

    claude -p "$(cat .claude/commands/fix-z3-api-types.md)" \
        --allowedTools 'Bash(*)' 'Read(*)' 'Write(*)' 'Edit(*)' 'Glob(*)' 'Grep(*)' \
        --verbose \
        2>&1 | tee -a "$LOG_FILE"

    EXIT_CODE=$?

    if [ $EXIT_CODE -eq 0 ]; then
        echo "$(date): Task completed successfully" | tee -a "$LOG_FILE"
        break
    fi

    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -lt $MAX_RETRIES ]; then
        echo "$(date): Claude exited with code $EXIT_CODE. Waiting 50 minutes before retry..." | tee -a "$LOG_FILE"
        sleep 3000
    fi
done

echo "$(date): Done. Check z3_api_fix_progress.md for results." | tee -a "$LOG_FILE"
