import os
import tempfile
import time
from fastapi import FastAPI, UploadFile, File
from fastapi.responses import HTMLResponse, JSONResponse
from faster_whisper import WhisperModel

app = FastAPI(title="Whisper Local Test Server")

print("⚡ Carregando modelo Whisper 'base' C++ (int8 CPU)...")
start_t = time.time()
model = WhisperModel("base", device="cpu", compute_type="int8")
print(f"✅ Modelo Whisper 'base' pronto em {time.time() - start_t:.2f}s!")

@app.get("/", response_class=HTMLResponse)
def get_index():
    with open("index.html", "r", encoding="utf-8") as f:
        return f.read()

@app.post("/transcribe")
async def transcribe_audio(file: UploadFile = File(...)):
    start_proc = time.time()
    try:
        filename = file.filename or "recording.wav"
        ext = "." + filename.split(".")[-1] if "." in filename else ".wav"
        with tempfile.NamedTemporaryFile(delete=False, suffix=ext) as tmp:
            content = await file.read()
            tmp.write(content)
            tmp_path = tmp.name

        # Vocabulário guiado de domínio (Domain Prompting) para Whisper em Português do Brasil
        domain_prompt = "Atendimento técnico comercial da empresa Tubarão Bombas em Minas Gerais. Vocabulário: Tubarão Bombas, poço artesiano, bomba solar, vazão em litros por hora, profundidade em metros, cacimba, represa, rio, reservatório, caixa d'água, voltagem, placas solares, Anauger, Schneider, Leão."

        segments, info = model.transcribe(
            tmp_path,
            language="pt",
            beam_size=5,
            initial_prompt=domain_prompt,
            vad_filter=True,
            vad_parameters=dict(min_silence_duration_ms=500)
        )
        text_segments = [seg.text.strip() for seg in segments]
        full_text = " ".join(text_segments)
        
        proc_time = time.time() - start_proc
        os.remove(tmp_path)

        print(f"🎙️ Transcrição concluída em {proc_time:.3f}s: \"{full_text}\"")

        return JSONResponse({
            "success": True,
            "text": full_text,
            "language": info.language,
            "probability": round(info.language_probability * 100, 1),
            "audio_duration_sec": round(info.duration, 2),
            "processing_time_sec": round(proc_time, 3)
        })
    except Exception as e:
        print("❌ Erro na transcrição:", e)
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)
