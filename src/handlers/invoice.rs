use std::str::FromStr;
use askama::Template;
use axum::body::Bytes;
use axum::extract::{Multipart, Path, RawForm};
use axum::response::{IntoResponse, Redirect};
use berechenbarkeit_lib::{InvoiceItemType, parse_pdf};
use crate::{AppError, HtmlTemplate};
use crate::db::{DatabaseConnection, DBCostCentre, DBInvoice, DBInvoiceItem};
use crate::handlers::cost_centre;


pub(crate) async fn invoice_add_upload(DatabaseConnection(mut conn): DatabaseConnection, mut multipart: Multipart) -> Result<Redirect, AppError> {
    let mut file: Option<Bytes> = None;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        if name == "file" {
            file = Some(data);
        }
    }

    let file = file.unwrap();
    // This error should never happen, as we have the HTTP form under our control
    let parsed_invoice = parse_pdf(&(file))?;



    let invoice_id = DBInvoice::insert(parsed_invoice.clone().into(), &mut conn).await?;

    DBInvoiceItem::bulk_insert(&mut conn, (parsed_invoice.items).into_iter().map(|i| DBInvoiceItem {
        id: None,
        invoice_id,
        typ: match i.typ {
            InvoiceItemType::Credit => "Credit".to_string(),
            InvoiceItemType::Expense => "Expense".to_string()
        },
        description: i.description.clone(),
        amount: i.amount,
        net_price_single: i.net_price_single,
        net_price_total: i.net_total_price,
        vat: i.vat,
        cost_centre_id: None,
        cost_centre: None,
    }).collect()).await?;

    // let mut fileio = File::create(format!("{}/invoice-{}.pdf", app_context.config.storage_base_path, invoice_id))?;
    // fileio.write_all(&file)?;

    Ok(Redirect::to(&format!("/invoice/{}/edit", invoice_id)))
}


#[derive(Template)]
#[template(path = "invoice/edit.html")]
struct InvoiceEditTemplate {
    invoice: DBInvoice,
    invoice_items: Vec<DBInvoiceItem>,
    cost_centres: Vec<DBCostCentre>
}

pub(crate) async fn invoice_edit(DatabaseConnection(mut conn): DatabaseConnection, Path(invoice_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let invoice = DBInvoice::get_by_id(invoice_id, &mut conn).await?;
    let invoice_items = DBInvoiceItem::get_by_invoice_id(invoice_id, &mut conn).await?;
    let cost_centres = DBCostCentre::get_all(&mut conn).await?;

    Ok(HtmlTemplate(InvoiceEditTemplate {
        invoice,
        invoice_items,
        cost_centres
    }))
}


pub(crate) async fn invoice_edit_submit(DatabaseConnection(mut conn): DatabaseConnection, Path(invoice_id): Path<i64>, RawForm(form): RawForm) -> Result<Redirect, AppError> {
    let form_data = serde_html_form::from_bytes::<Vec<(String, String)>>(&form)?;

    for form_field in form_data.into_iter() {
        // TODO: Use bulk UPDATE
        let mut cost_centre = None;
        if !form_field.1.is_empty() {
            cost_centre = Some(i64::from_str(&form_field.1)?);
        }
        DBInvoiceItem::update_cost_centre(i64::from_str(&form_field.0)?, cost_centre, &mut conn).await?;
    }

    Ok(Redirect::to(&format!("/invoice/{}/edit", invoice_id)))
}


#[derive(Template)]
#[template(path = "invoice/list.html")]
struct InvoiceListTemplate {
    invoices: Vec<DBInvoice>,
}

pub(crate) async fn invoice_list(DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    let invoices = DBInvoice::get_all(&mut conn).await?;
    Ok(HtmlTemplate(InvoiceListTemplate { invoices }))

}
