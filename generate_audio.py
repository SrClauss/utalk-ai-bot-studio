import asyncio
import edge_tts
import os

TEXTS = {
    "saudacao_bom_dia": "Olá, bom dia! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
    "saudacao_boa_tarde": "Olá, boa tarde! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
    "saudacao_boa_noite": "Olá, boa noite! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
    "poco_cacimba_detalhes": "Entendido! Para poço artesiano ou cacimba, você saberia me informar qual é a profundidade do poço e a distância até a caixa d'água ou reservatório?",
    "rio_represa_detalhes": "Entendido! Para captação em rio, represa ou açude, você saberia me informar qual é a altura de subida do terreno e a distância até a caixa d'água ou reservatório?",
    "profundidade_distancia": "Entendido! E você saberia me informar qual é a profundidade ou altura da água e a distância até a caixa d'água ou reservatório?",
    "vazao_agua": "Perfeito! E quantos litros de água você precisa abastecer por dia ou por hora?",
    "encaminhamento_resumo": "Excelente! Já compilei todas as informações do seu projeto e estou encaminhando agora mesmo para a nossa equipe de especialistas. Um de nossos atendentes vai te chamar em instantes para finalizar o seu orçamento. Muito obrigado!"
}

VOICE = "pt-BR-AntonioNeural"

async def main():
    os.makedirs("assets", exist_ok=True)
    for key, text in TEXTS.items():
        output_file = f"assets/{key}.mp3"
        print(f"Generating {output_file}...")
        communicate = edge_tts.Communicate(text, VOICE)
        await communicate.save(output_file)
        print(f"Saved {output_file}")

if __name__ == "__main__":
    asyncio.run(main())
