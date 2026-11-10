<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { commands } from "$lib/bindings";

  function formatPrice(value: number): string {
    if (value == null || isNaN(value)) return "0.00 DZD";
    return (
      new Intl.NumberFormat("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      }).format(value) + " DZD"
    );
  }

  // دالة التحقق: تمنع الحروف العربية وتسمح باللاتينية والفرنسية (مع التشكيل)
  function isValidLatin(text: string): boolean {
    // نطاق الحروف العربية في Unicode هو \u0600-\u06FF
    const arabicRegex = /[\u0600-\u06FF]/;
    return !arabicRegex.test(text);
  }

  let clientName = $state("");
  let items = $state([
    { name: "", price: 0, receipt_image_path: null, note: "" },
  ]);
  let laborFee = $state(0);
  let transportFee = $state(0);
  let date = $state("");
  let error = $state("");
  let success = $state("");
  let parts = $state<any[]>([]);
  let isSaving = $state(false);

  onMount(async () => {
    const dateResult = await commands.currentDateDmy();
    if (dateResult.status === "ok") {
      date = dateResult.data;
    }
    await loadParts();
  });

  async function loadParts() {
    try {
      const result = await commands.getParts();
      if (result.status === "ok") {
        parts = result.data;
      }
    } catch (e) {
      console.error("Failed to load parts:", e);
    }
  }

  function addItem() {
    items = [
      ...items,
      { name: "", price: 0, receipt_image_path: null, note: "" },
    ];
  }

  function removeItem(index: number) {
    items = items.filter((_, i) => i !== index);
  }

  function selectPart(index: number, partName: string) {
    const part = parts.find((p) => p.name === partName);
    if (part) {
      if (part.low_bound != null && part.high_bound != null) {
        items[index].price = (part.low_bound + part.high_bound) / 2;
      } else if (part.low_bound != null) {
        items[index].price = part.low_bound;
      } else if (part.high_bound != null) {
        items[index].price = part.high_bound;
      }
    }
  }

  function getPartHint(item: any) {
    const part = parts.find((p) => p.name === item.name);
    if (!part) return null;
    if (part.low_bound != null && part.high_bound != null) {
      return `السعر المعتاد: ${part.low_bound} - ${part.high_bound} دج`;
    } else if (part.low_bound != null) {
      return `الحد الأدنى: ${part.low_bound} دج`;
    } else if (part.high_bound != null) {
      return `الحد الأقصى: ${part.high_bound} دج`;
    }
    return null;
  }

  async function saveInvoice() {
    if (!clientName.trim()) {
      error = "الرجاء إدخال اسم العميل";
      return;
    }

    // تطبيق قاعدة منع العربية
    if (!isValidLatin(clientName)) {
      error = "اسم العميل يجب أن يكون بالأحرف اللاتينية فقط (مثل: Ahmed). يرجى تصحيح الاسم.";
      return;
    }

    const validItems = items.filter((item) => item.name.trim());
    if (validItems.length === 0) {
      error = "الرجاء إضافة قطعة واحدة على الأقل";
      return;
    }

    isSaving = true;
    error = "";
    success = "";

    try {
      // نستدعي الأمر الجديد الذي يحفظ ويولد الـ PDF
      const result = await commands.saveAndGeneratePdf(
        {
          client_name: clientName.trim(),
          items: validItems,
          labor_fee: laborFee,
          transport_fee: transportFee,
        },
        date,
      );

      if (result.status === "ok") {
        success = "✅ تم حفظ الفاتورة وإنشاء PDF بنجاح! المسار: " + result.data;
        setTimeout(() => {
          clientName = "";
          items = [{ name: "", price: 0, receipt_image_path: null, note: "" }];
          laborFee = 0;
          transportFee = 0;
          success = "";
        }, 5000);
      } else {
        error = mapError(result.error);
      }
    } catch (e) {
      error = "خطأ في الاتصال: " + String(e);
    } finally {
      isSaving = false;
    }
  }

  function mapError(err: any) {
    const errStr = JSON.stringify(err);
    if (errStr.includes("InvalidTotal")) return "مبلغ الفاتورة غير صالح";
    if (errStr.includes("DatabaseError")) return "خطأ في قاعدة البيانات";
    if (errStr.includes("IoError")) return "خطأ في النظام";
    if (errStr.includes("PdfGenerationFailed")) return "فشل إنشاء ملف PDF";
    return "حدث خطأ غير متوقع";
  }

  let total = $derived(
    items.reduce((sum, item) => sum + (Number(item.price) || 0), 0) +
      Number(laborFee) +
      Number(transportFee),
  );
</script>

<div class="container">
  <div class="header">
    <h1>إنشاء فاتورة جديدة</h1>
    <button class="manage-parts-btn" onclick={() => goto("/parts")}>إدارة القطع</button>
  </div>
  <div class="date-display">التاريخ: {date}</div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if success}
    <div class="success">{success}</div>
  {/if}

  <div class="form-section">
    <div class="form-group">
      <label>اسم العميل (للفاتورة باللاتينية) *</label>
      <input
        type="text"
        bind:value={clientName}
        placeholder="Ex: Ahmed"
        dir="ltr"
        style="text-align: left;"
      />
      <small style="color: #d16a6c; font-size: 0.8rem; margin-top: 4px; display: block;">
        يرجى كتابة الاسم بالأحرف اللاتينية. (لن يتم قبول الحروف العربية).
      </small>
    </div>
  </div>

  <div class="form-section">
    <h2>القطع المستخدمة</h2>

    {#each items as item, index}
      <div class="item-card">
        <div class="item-header">
          <span>قطعة #{index + 1}</span>
          {#if items.length > 1}
            <button class="remove-btn" onclick={() => removeItem(index)}>حذف</button>
          {/if}
        </div>

        <div class="form-group">
          <label>اختر القطعة (باللاتينية)</label>
          <select
            bind:value={item.name}
            dir="auto"
            onchange={(e: Event) =>
              selectPart(index, (e.target as HTMLSelectElement).value)}
          >
            <option value="">-- اختر قطعة --</option>
            {#each parts as part}
              <option value={part.name}>{part.name}</option>
            {/each}
          </select>
          <small style="color: #6c6d6c; font-size: 0.8rem; margin-top: 4px; display: block;">
            تذكر: الفاتورة تطبع باللاتينية.
          </small>
        </div>

        {#if item.name}
          <div class="form-group">
            <label>السعر (دج)</label>
            <input type="number" bind:value={item.price} step="0.01" min="0" />
            {#if getPartHint(item)}
              <div class="hint">{getPartHint(item)}</div>
            {/if}
          </div>

          <div class="form-group">
            <label>ملاحظة (اختياري)</label>
            <input
              type="text"
              bind:value={item.note}
              placeholder="أضف ملاحظة إذا لزم الأمر"
              dir="auto"
            />
          </div>
        {/if}
      </div>
    {/each}

    <button class="add-item-btn" onclick={addItem}>+ إضافة قطعة أخرى</button>
  </div>

  <div class="form-section">
    <h2>الرسوم الإضافية</h2>

    <div class="form-row">
      <div class="form-group">
        <label>أجرة العمل (دج)</label>
        <input type="number" bind:value={laborFee} step="0.01" min="0" />
      </div>

      <div class="form-group">
        <label>أجرة النقل (دج)</label>
        <input type="number" bind:value={transportFee} step="0.01" min="0" />
      </div>
    </div>
  </div>

  <div class="total-display">
    <span>المجموع الكلي:</span>
    <span class="total-amount">{formatPrice(total)}</span>
  </div>

  <button class="save-btn" onclick={saveInvoice} disabled={isSaving}>
    {isSaving ? "⏳ جاري الحفظ والتوليد..." : "حفظ وإصدار الفاتورة"}
  </button>
</div>

<style>
  .container {
    max-width: 750px;
    margin: 0 auto;
    padding: 1.5rem;
    direction: rtl;
    color: #6c6d6c;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.25rem;
  }

  h1, h2 {
    color: #6f3c39;
    margin: 0;
  }

  h1 { font-size: 1.9rem; font-weight: 800; letter-spacing: -0.5px; }
  h2 { font-size: 1.3rem; font-weight: 700; margin-bottom: 0.75rem; }

  .manage-parts-btn, .save-btn {
    background: #f14345;
    color: white;
    border: 2px solid #6f3c39;
    padding: 0.55rem 1.1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 700;
    box-shadow: 4px 4px 0px 0px #6f3c39;
    transition: all 0.1s ease;
  }

  .manage-parts-btn:active, .save-btn:active {
    transform: translate(4px, 4px);
    box-shadow: 0px 0px 0px 0px #6f3c39;
  }

  .save-btn {
    width: 100%;
    padding: 0.85rem;
    font-size: 1.1rem;
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
    box-shadow: 4px 4px 0px 0px #6f3c39;
  }

  input, select {
    width: 100%;
    padding: 0.7rem;
    border: 2px solid #6f3c39;
    border-radius: 6px;
    font-size: 0.95rem;
    background: #e3e3db !important;
    box-sizing: border-box;
    color: #6c6d6c;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  input:focus, select:focus {
    outline: none;
    background: #ffffff !important;
    box-shadow: 4px 4px 0px 0px rgba(241, 67, 69, 0.3);
  }

  .hint {
    color: #d16a6c;
    font-size: 0.8rem;
    margin-top: 0.25rem;
    font-weight: 600;
  }

  .date-display {
    background: #FDFCF8;
    padding: 0.6rem 0.75rem;
    border-radius: 6px;
    margin-bottom: 1.25rem;
    font-weight: 700;
    font-size: 0.95rem;
    border: 2px solid #6f3c39;
    box-shadow: 4px 4px 0px 0px #6f3c39;
  }

  .error, .success {
    padding: 0.85rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    border: 2px solid;
    font-weight: 600;
    font-size: 0.9rem;
  }
  .error { background: #fee; color: #c33; border-color: #c33; }
  .success { background: #efe; color: #363; border-color: #363; }

  .form-section {
    background: #FDFCF8;
    padding: 1.25rem;
    border-radius: 8px;
    border: 2px solid #6f3c39;
    box-shadow: 6px 6px 0px 0px #6f3c39;
    margin-bottom: 1.25rem;
  }

  .form-group { margin-bottom: 0.85rem; }
  .form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 0.85rem; }

  label {
    display: block;
    margin-bottom: 0.4rem;
    font-weight: 700;
    font-size: 0.9rem;
  }

  .item-card {
    border: 2px solid #6f3c39;
    border-radius: 6px;
    padding: 0.85rem;
    margin-bottom: 0.85rem;
    background: #ffffff;
    box-shadow: 3px 3px 0px 0px #6f3c39;
  }

  .item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.85rem;
    font-weight: 700;
    font-size: 0.95rem;
  }

  .remove-btn {
    background: transparent;
    color: #f14345;
    border: 2px solid #f14345;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 700;
    transition: all 0.1s;
  }

  .remove-btn:active {
    transform: translate(2px, 2px);
  }

  .add-item-btn {
    width: 100%;
    background: #e3e3db;
    color: #6f3c39;
    border: 2px dashed #6f3c39;
    padding: 0.65rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 700;
    transition: all 0.1s;
  }

  .add-item-btn:active {
    transform: translate(2px, 2px);
  }

  .total-display {
    background: #6f3c39;
    color: white;
    padding: 1.1rem 1.25rem;
    border-radius: 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 1.1rem;
    font-weight: 700;
    margin-bottom: 1.25rem;
    border: 2px solid #6f3c39;
    box-shadow: 6px 6px 0px 0px #1a1a1a;
  }

  .total-amount {
    font-size: 1.5rem;
    color: #f14345;
    font-weight: 800;
    text-shadow: 1px 1px 0px rgba(0,0,0,0.2);
  }
</style>
