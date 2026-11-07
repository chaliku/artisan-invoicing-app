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

    const validItems = items.filter((item) => item.name.trim());
    if (validItems.length === 0) {
      error = "الرجاء إضافة قطعة واحدة على الأقل";
      return;
    }

    isSaving = true;
    error = "";
    success = "";

    try {
      const result = await commands.saveInvoice(
        {
          client_name: clientName.trim(),
          items: validItems,
          labor_fee: laborFee,
          transport_fee: transportFee,
        },
        date,
      );

      if (result.status === "ok") {
        success = "تم حفظ الفاتورة بنجاح";
        setTimeout(() => {
          clientName = "";
          items = [{ name: "", price: 0, receipt_image_path: null, note: "" }];
          laborFee = 0;
          transportFee = 0;
          success = "";
        }, 3000);
      } else {
        error = mapError(result.error);
      }
    } catch (e) {
      error = "خطأ في الاتصال";
    } finally {
      isSaving = false;
    }
  }

  function mapError(err: any) {
    const errStr = JSON.stringify(err);
    if (errStr.includes("InvalidTotal")) return "مبلغ الفاتورة غير صالح";
    if (errStr.includes("DatabaseError")) return "خطأ في قاعدة البيانات";
    if (errStr.includes("IoError")) return "خطأ في النظام";
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
    <button class="manage-parts-btn" onclick={() => goto("/parts")}
      >إدارة القطع</button
    >
  </div>

  <!-- زر الاختبار المؤقت -->
  <button
    style="width: 100%; background: #6f3c39; color: white; padding: 1rem; border-radius: 8px; cursor: pointer; margin-bottom: 1.5rem; font-size: 1rem; font-weight: bold; border: none;"
    onclick={async () => {
      try {
        const dummyInvoice = {
          client_name: "Ahmed",
          items: [
            {
              name: "Robinet",
              price: 500.0,
              receipt_image_path: null,
              note: null,
            },
          ],
          labor_fee: 1500.0,
          transport_fee: 200.0,
          total: 2200.0,
        };
        const res = await commands.generateInvoicePdf(
          dummyInvoice,
          "Facture",
          "07/11/2026",
        );
        if (res.status === "ok") {
          alert("نجاح! الملف في: " + res.data);
        } else {
          alert("خطأ: " + JSON.stringify(res.error));
        }
      } catch (e) {
        alert("خطأ في الاتصال: " + String(e));
      }
    }}
  >
    اختبار التصميم الاحترافي (PDF)
  </button>

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
      <small
        style="color: #d16a6c; font-size: 0.8rem; margin-top: 4px; display: block;"
      >
        يرجى كتابة الاسم بالأحرف اللاتينية لكي يظهر بشكل صحيح في الفاتورة
        المطبوعة.
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
            <button class="remove-btn" onclick={() => removeItem(index)}
              >حذف</button
            >
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
          <small
            style="color: #6c6d6c; font-size: 0.8rem; margin-top: 4px; display: block;"
          >
            تذكر: الفاتورة تطبع باللاتينية. استخدم صورة القطعة لكتابة الاسم
            العربي إن لزم.
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
    {isSaving ? "⏳ جاري الحفظ..." : "حفظ الفاتورة"}
  </button>
</div>

<style>
  /* لا نحتاج :global(body) هنا لأننا في +layout.svelte */

  .container {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
    direction: rtl;
    color: #6c6d6c;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  h1,
  h2 {
    color: #6c6d6c;
    margin: 0;
  }

  h1 {
    font-size: 2rem;
  }
  h2 {
    font-size: 1.5rem;
    margin-bottom: 1rem;
  }

  .manage-parts-btn,
  .save-btn {
    background: #f14345;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 600;
    transition: opacity 0.2s;
  }

  .manage-parts-btn:hover,
  .save-btn:hover {
    opacity: 0.85;
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* الحقول: كريمي افتراضي، أبيض عند التركيز */
  input,
  select {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #d16a6c;
    border-radius: 4px;
    font-size: 1rem;
    background: #e3e3db !important;
    box-sizing: border-box;
    color: #6c6d6c;
    transition: all 0.2s ease;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: #f14345;
    background: #ffffff !important;
    box-shadow: 0 0 0 3px rgba(241, 67, 69, 0.1);
  }

  .hint {
    color: #d16a6c;
    font-size: 0.875rem;
    margin-top: 0.25rem;
  }

  /* الحاويات: أبيض دافئ #FDFCF8 */
  .date-display {
    background: #fdfcf8;
    padding: 0.75rem;
    border-radius: 4px;
    margin-bottom: 1.5rem;
    font-weight: 500;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  }

  .error,
  .success {
    padding: 1rem;
    border-radius: 4px;
    margin-bottom: 1rem;
  }
  .error {
    background: #fee;
    color: #c33;
    border: 1px solid #fcc;
  }
  .success {
    background: #efe;
    color: #363;
    border: 1px solid #cfc;
  }

  .form-section {
    background: #fdfcf8; /* الأبيض الدافئ */
    padding: 1.5rem;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
    margin-bottom: 1.5rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }
  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
  }

  /* البطاقة الداخلية: بيضاء ناصعة لتبرز داخل الحاوية الدافئة */
  .item-card {
    border: 1px solid #e3e3db;
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 1rem;
    background: #ffffff;
  }

  .item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    font-weight: 600;
  }

  .remove-btn {
    background: transparent;
    color: #f14345;
    border: 1px solid #f14345;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .remove-btn:hover {
    background: rgba(241, 67, 69, 0.1);
  }

  .add-item-btn {
    width: 100%;
    background: transparent;
    color: #f14345;
    border: 2px dashed #f14345;
    padding: 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 600;
  }

  .add-item-btn:hover {
    background: rgba(241, 67, 69, 0.05);
  }

  .total-display {
    background: #6f3c39;
    color: white;
    padding: 1.5rem;
    border-radius: 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 1.5rem;
  }

  .total-amount {
    font-size: 1.5rem;
    color: #d16a6c;
  }
</style>
