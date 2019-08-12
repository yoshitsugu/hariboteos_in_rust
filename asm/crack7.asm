[BITS 32]

		GLOBAL	hrmain

[SECTION .text]

hrmain:
		MOV		AX,1005*8
		MOV		DS,AX
		CMP		DWORD [DS:0x0004],'Hari'
		JNE		fin					; アプリではないようなので何もしない

		MOV		ECX,[DS:0x0000]		; このアプリのデータセグメントの大きさを読み取る
		MOV		AX,2005*8
		MOV		DS,AX

crackloop:							; 123で埋め尽くす
		ADD		ECX,-1
		MOV		BYTE [DS:ECX],123
		CMP		ECX,0
		JNE		crackloop

fin:								; 終了
		MOV		EDX,4
		INT		0x40
