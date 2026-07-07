# -*- coding: utf-8 -*-
# 한글에서 74312 각 본문 문단의 페이지(SetPos+current_page)를 추출 → paramap.tsv
import shutil, os
_o = shutil.rmtree
shutil.rmtree = lambda p, *a, **k: None if 'gen_py' in str(p) else _o(p, *a, **k)
import subprocess
subprocess.run(['taskkill', '/F', '/IM', 'Hwp.exe'], capture_output=True)
from pyhwpx import Hwp

SRC = r'C:\Users\planet\hwpdocs\opinion_downloads\중소벤처기업부\74312_(법령안) 벤처투자 촉진에 관한 법률 시행규칙 일부개정령(안).hwpx'
OUT = r'C:\Users\planet\t\rhwp\output\poc\task2019\paramap.tsv'
hwp = Hwp(new=True, visible=False)
hwp.open(SRC)
hwp.MoveDocEnd()
maxpara = hwp.GetPos()[1]
with open(OUT, 'w', encoding='utf-8') as f:
    f.write('pi\thwp_page\n')
    for para in range(maxpara + 1):
        hwp.SetPos(0, para, 0)
        f.write(f'{para}\t{hwp.current_page}\n')
hwp.clear(option=1)
hwp.quit()
subprocess.run(['taskkill', '/F', '/IM', 'Hwp.exe'], capture_output=True)
print('DONE maxpara=', maxpara)
