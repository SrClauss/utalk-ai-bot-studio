FROM python:3.11-slim

RUN apt-get update && apt-get install -y ffmpeg git build-essential wget && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir faster-whisper fastapi uvicorn python-multipart

WORKDIR /app

RUN python3 -c "from faster_whisper import WhisperModel; WhisperModel('base', device='cpu', compute_type='int8')"

COPY app.py /app/app.py
COPY index.html /app/index.html

EXPOSE 8080

CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8080"]
